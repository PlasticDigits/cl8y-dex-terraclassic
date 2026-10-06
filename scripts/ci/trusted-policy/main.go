// Trusted configuration for the DEX only. Never evaluate PR configuration or text.
package main

import (
	"crypto/ed25519"
	"crypto/sha256"
	"crypto/x509"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/yaronf/httpsign"
	"gopkg.in/yaml.v3"
)

const signatureName = "woodpecker-ci-extensions"
const maxRequestBytes = 1 << 20

type configFile struct {
	Name string `json:"name"`
	Data string `json:"data"`
}

// bundle embeds the reviewed runner and scanner into commands supplied by the
// control plane, so changing/deleting either file in a PR cannot disable them.
func bundle(dir string) ([]byte, error) {
	raw, err := os.ReadFile(filepath.Join(dir, ".woodpecker.yaml"))
	if err != nil {
		return nil, err
	}
	var workflow map[string]any
	if err := yaml.Unmarshal(raw, &workflow); err != nil {
		return nil, err
	}
	steps, ok := workflow["steps"].([]any)
	if !ok || len(steps) == 0 {
		return nil, fmt.Errorf("missing steps")
	}
	assets := []struct{ source, target string }{
		{"scripts/ci/functional-run.sh", "/tmp/dex-policy/functional-run.sh"},
		{"scripts/ci/gitleaks-scan-tracked.sh", "/tmp/dex-policy/gitleaks-scan-tracked.sh"},
		{".gitleaks.toml", "/tmp/dex-policy/gitleaks.toml"},
	}
	prefix := "set -eu\nmkdir -p /tmp/dex-policy\n"
	for _, asset := range assets {
		body, err := os.ReadFile(filepath.Join(dir, asset.source))
		if err != nil {
			return nil, err
		}
		delimiter := fmt.Sprintf("DEX_POLICY_%x", sha256.Sum256(body))
		prefix += fmt.Sprintf("cat > %s <<'%s'\n%s\n%s\n", asset.target, delimiter, body, delimiter)
	}
	prefix += "export GITLEAKS_SCAN_SCRIPT=/tmp/dex-policy/gitleaks-scan-tracked.sh GITLEAKS_CONFIG=/tmp/dex-policy/gitleaks.toml\n"
	// Woodpecker interpolates configuration before the shell sees it. Escape
	// embedded dollar signs so Bash arrays/defaults and command substitutions
	// survive that phase and are evaluated only inside the isolated job.
	prefix = strings.ReplaceAll(prefix, "$", "$$")
	for _, value := range steps {
		step, ok := value.(map[string]any)
		if !ok {
			return nil, fmt.Errorf("invalid step")
		}
		commands, ok := step["commands"].([]any)
		if !ok || len(commands) == 0 {
			return nil, fmt.Errorf("missing commands")
		}
		for i, value := range commands {
			command, ok := value.(string)
			if !ok {
				return nil, fmt.Errorf("invalid command")
			}
			commands[i] = strings.ReplaceAll(command, "scripts/ci/functional-run.sh", "/tmp/dex-policy/functional-run.sh")
		}
		step["commands"] = append([]any{prefix}, commands...)
	}
	data, err := yaml.Marshal(workflow)
	if err != nil {
		return nil, err
	}
	return json.Marshal(map[string]any{"configs": []configFile{{Name: ".woodpecker.yaml", Data: string(data)}}})
}

func handler(key ed25519.PublicKey, policy []byte) (http.Handler, error) {
	verifier, err := httpsign.NewEd25519Verifier(key, httpsign.NewVerifyConfig().SetMaxBodySize(maxRequestBytes), httpsign.Headers("@request-target", "content-digest"))
	if err != nil {
		return nil, err
	}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprintf(w, "{\"policy_sha256\":\"%x\"}\n", sha256.Sum256(policy))
	})
	endpoint := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var request struct {
			Repo struct {
				ID       int    `json:"id"`
				FullName string `json:"full_name"`
				ForgeURL string `json:"forge_url"`
			} `json:"repo"`
			Netrc json.RawMessage `json:"netrc"`
		}
		if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
			http.Error(w, "invalid request", http.StatusBadRequest)
			return
		}
		if request.Repo.ID != 7 || request.Repo.FullName != "code/cl8y-dex-terraclassic" || request.Repo.ForgeURL != "https://git.cl8y.com/code/cl8y-dex-terraclassic" {
			http.Error(w, "repository refused", http.StatusForbidden)
			return
		}
		if len(request.Netrc) > 0 && string(request.Netrc) != "null" {
			http.Error(w, "credential forwarding refused", http.StatusForbidden)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write(policy)
	})
	config := httpsign.NewHandlerConfig().SetMaxBodySize(maxRequestBytes).
		SetFetchVerifier(func(_ *http.Request) (string, *httpsign.Verifier) { return signatureName, verifier }).
		SetLogger(nil)
	verified := httpsign.WrapHandler(endpoint, *config)
	mux.Handle("POST /config", http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		r.Body = http.MaxBytesReader(w, r.Body, maxRequestBytes)
		verified.ServeHTTP(w, r)
	}))
	return mux, nil
}

func readKey(path string) (ed25519.PublicKey, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	block, rest := pem.Decode(raw)
	if block == nil || len(strings.TrimSpace(string(rest))) != 0 {
		return nil, fmt.Errorf("invalid public key PEM")
	}
	parsed, err := x509.ParsePKIXPublicKey(block.Bytes)
	if err != nil {
		return nil, err
	}
	key, ok := parsed.(ed25519.PublicKey)
	if !ok {
		return nil, fmt.Errorf("Ed25519 public key required")
	}
	return key, nil
}

func main() {
	if len(os.Args) == 2 && os.Args[1] == "health" {
		client := http.Client{Timeout: 2 * time.Second}
		response, err := client.Get("http://127.0.0.1:8080/health")
		if err != nil {
			os.Exit(1)
		}
		response.Body.Close()
		if response.StatusCode != http.StatusOK {
			os.Exit(1)
		}
		return
	}
	if len(os.Args) == 3 && os.Args[1] == "bundle" {
		data, err := bundle(os.Args[2])
		if err != nil {
			log.Fatal(err)
		}
		os.Stdout.Write(data)
		return
	}
	policy, err := os.ReadFile("/policy/config.json")
	if err != nil {
		log.Fatal("policy unavailable")
	}
	key, err := readKey("/policy/woodpecker-public.pem")
	if err != nil {
		log.Fatal("public key unavailable or invalid")
	}
	h, err := handler(key, policy)
	if err != nil {
		log.Fatal("signature verifier unavailable")
	}
	server := &http.Server{Addr: ":8080", Handler: h, ReadHeaderTimeout: 5 * time.Second, ReadTimeout: 10 * time.Second, WriteTimeout: 10 * time.Second, IdleTimeout: 30 * time.Second, MaxHeaderBytes: 16 << 10}
	log.Printf("trusted DEX policy sha256=%x", sha256.Sum256(policy))
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatal("server stopped")
	}
}
