package main

import (
	"bytes"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yaronf/httpsign"
)

func TestSignedPolicyIgnoresContributorConfiguration(t *testing.T) {
	public, private, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	policy := []byte(`{"configs":[{"name":".woodpecker.yaml","data":"trusted required jobs"}]}`)
	h, err := handler(public, policy)
	if err != nil {
		t.Fatal(err)
	}
	server := httptest.NewServer(h)
	defer server.Close()
	signer, err := httpsign.NewEd25519Signer(private, httpsign.NewSignConfig(), httpsign.Headers("@request-target", "content-digest"))
	if err != nil {
		t.Fatal(err)
	}
	client := httpsign.NewClient(http.Client{}, httpsign.NewClientConfig().SetSignatureName(signatureName).SetSigner(signer))
	for _, injected := range []string{"", "steps: []", "steps: [{name: contracts, commands: ['true']}]", "cache: {key: trusted-main}"} {
		t.Run(injected, func(t *testing.T) {
			body, _ := json.Marshal(map[string]any{"repo": map[string]any{"id": 7, "full_name": "code/cl8y-dex-terraclassic", "forge_url": "https://git.cl8y.com/code/cl8y-dex-terraclassic"}, "configuration": []map[string]string{{"name": ".woodpecker.yaml", "data": injected}}})
			req, _ := http.NewRequest(http.MethodPost, server.URL+"/config", bytes.NewReader(body))
			response, err := client.Do(req)
			if err != nil {
				t.Fatal(err)
			}
			defer response.Body.Close()
			result, _ := io.ReadAll(response.Body)
			if response.StatusCode != 200 || !bytes.Equal(result, policy) {
				t.Fatalf("status=%d policy replaced", response.StatusCode)
			}
		})
	}
	for _, body := range []string{
		`{"repo":{"id":8,"full_name":"code/cl8y-dex-terraclassic"}}`,
		`{"repo":{"id":7,"full_name":"code/cl8y-dex-terraclassic","forge_url":"https://git.cl8y.com/code/cl8y-dex-terraclassic"},"netrc":{"password":"synthetic-only"}}`,
	} {
		req, _ := http.NewRequest(http.MethodPost, server.URL+"/config", strings.NewReader(body))
		res, err := client.Do(req)
		if err != nil {
			t.Fatal(err)
		}
		res.Body.Close()
		if res.StatusCode != 403 {
			t.Fatalf("unsafe request accepted: %d", res.StatusCode)
		}
	}
	res, err := http.Post(server.URL+"/config", "application/json", strings.NewReader(`{}`))
	if err != nil {
		t.Fatal(err)
	}
	res.Body.Close()
	if res.StatusCode != 401 {
		t.Fatalf("unsigned request accepted: %d", res.StatusCode)
	}
}

func TestBundleCopiesReviewedAssets(t *testing.T) {
	dir := t.TempDir()
	if err := os.MkdirAll(filepath.Join(dir, "scripts/ci"), 0700); err != nil {
		t.Fatal(err)
	}
	files := map[string]string{
		".woodpecker.yaml":                    "steps:\n  - name: contracts\n    commands: [bash scripts/ci/functional-run.sh contracts]\n",
		".gitleaks.toml":                      "reviewed-config",
		"scripts/ci/functional-run.sh":        "#!/bin/bash\necho reviewed-runner\necho \"${required[@]}\" \"$PWD\" \"$(cat state)\"",
		"scripts/ci/gitleaks-scan-tracked.sh": "#!/bin/bash\necho reviewed-scanner",
	}
	for name, value := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(value), 0600); err != nil {
			t.Fatal(err)
		}
	}
	data, err := bundle(dir)
	if err != nil {
		t.Fatal(err)
	}
	var result struct {
		Configs []configFile `json:"configs"`
	}
	if err := json.Unmarshal(data, &result); err != nil {
		t.Fatal(err)
	}
	for _, expected := range []string{"reviewed-runner", "reviewed-scanner", "reviewed-config", "$${required[@]}", "$$PWD", "$$(cat state)", "GITLEAKS_CONFIG=/tmp/dex-policy/gitleaks.toml", "bash /tmp/dex-policy/functional-run.sh contracts"} {
		if !strings.Contains(result.Configs[0].Data, expected) {
			t.Errorf("bundle missing %s", expected)
		}
	}
	os.Remove(filepath.Join(dir, ".woodpecker.yaml"))
	if _, err := bundle(dir); err == nil {
		t.Fatal("missing trusted config accepted")
	}
}
