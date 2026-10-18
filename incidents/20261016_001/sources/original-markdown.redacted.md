SOURCE NARRATIVE — incomplete/superseded. This is a redacted reporter/model-supplied account, not the corrected investigation finding. Host-log and attribution assertions are unverified. REPORT.md records the later chain evidence and scope correction. Operator requests and proposed next steps are omitted.

# CL8Y Bot Wallets — Security Incident Report

Oct 15, 2026 · [VICTIM_REPORTER]

## Summary

Between 05:19 and 06:32 BST on 15 October 2026, transactions not sent by our trading bot were signed with the keys of our two CL8Y DEX bot wallets. Three CL8Y bridge deposits moved about **$590 of stablecoins from Terra Classic to BSC**, to two EVM addresses we are treating as unauthorized.

- **Bridged out:** 237.37 + 342.71 USDT and 10.18 vFDUSD (bridge nonces **152, 153, 154**).


[Operator response requests omitted from this factual archive.]
## Affected wallets

Both wallets are bot-controlled hot wallets on Terra Classic (columbus-5). Every transaction below carries that wallet's own signature.

| Wallet | Terra address | Signer public key (prefix) | Role |
| --- | --- | --- | --- |
| B (main) | `terra12z8vz3vgnalt8trykd6nntdyp54e95yqsdtjj3` | `AwBUOxtp…` | Market making, window arb |
| A (arb) | `terra13w2zl4cetngp5cssw0uhkkxryrxn5wqfdgrp2h` | `AsFgOgzK…` | Arbitrage |

Wallet B's signer key is the same one that signs the bot's normal trades, so these were signed with the real key, not a lookalike.

## Bridge transfers out

All three went through the CL8Y bridge `terra18m02l2f43c2dagqnz3kfccpgz9pzzz5hk9l5mh5wvr6dcvv47zfqdfs7la` via `deposit_cw20_mintable_burn`, destination chain `0x00000038` (BSC, chain ID 56).

| Nonce | Time (UTC) | From wallet | Token | Sent | Bridged (after fee) | BSC destination |
| --- | --- | --- | --- | --- | --- | --- |
| 154 | 2026-10-15 04:43:17 | B | USDT | 342.710552514545617774 | 341.682420857001980921 | `0x46b2ebf5ec9bbbca91fb826a6c13aa7ccdb66f8d` |
| 153 | 2026-10-15 04:43:11 | B | vFDUSD | 10.184892 | 10.154338 | `0x377a56e0fea12d5e0632429326b5cd9e47e10e5b` |
| 152 | 2026-10-15 04:19:38 | A | USDT | 237.366032711296597552 | 236.653934613162707760 | `0x46b2ebf5ec9bbbca91fb826a6c13aa7ccdb66f8d` |

**Terra transaction hashes**

- Nonce 154: `FDDED73075377ADA5C102C76E53C08FBAB331EA934699CC2D2C3B42C852BF1D7`
- Nonce 153: `3A5D25F06E769EE61E2E8AC271646A8E0F9F57828102B054F2FB49F635286BAA`
- Nonce 152: `E76AF7A330D76A60EC0557347724E2405B07CF1B4878140EAA722AD400A093E7`

**Token contracts:** USDT `terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4` · vFDUSD `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3`. The bridge fee was paid to `terra1q7txczaxuvy923k4km9ya062dryk6mjwd6tmzm` on each.

Note: two different BSC addresses were used, and both USDT deposits went to the same one.

## Timeline of unauthorized transactions

Ten transactions over 73 minutes, all successful (code 0). Times are UTC; add one hour for BST.

| Time (UTC) | Wallet | Action | Detail | Tx hash |
| --- | --- | --- | --- | --- |
| 05:32:30 | A | Unwrap | 500,780.517182 cLUNC unwrapped; LUNC paid back to wallet A | `16B4C16E1417B180C1389459BC9D9C86E5E63CE98AC5DCCA946CF5282C8110B7` |
| 05:00:58 | B | Unwrap | 1,460,082.990597 cLUNC unwrapped; 1,452,636.57 LUNC paid back to wallet B | `9BAB7E8323A12DDB6F59274C54D05E3FA08926CCA7D2F7481CE90D55826E87DB` |
| 05:00:53 | B | LP withdrawal | Pair `terra1q5karg99p5hc0jvplgn2fd8akg39r4ss00cu5rwkm600yqvmssgqhwqvq0`; 122.51 CL8Y + 1,460,082.99 cLUNC returned to wallet B | `E0C7D739846CDC1A2482C40227242D111BAD7177E9D7E63C500213266D9BA640` |
| 04:57:14 | B | Swap | 20,147.874702 cUSTC → 116.11 UST1 | `CEDC3D29AAB84E9FBE6FA98587FAC54A93E677650956AEEBB2884DFB45D875C9` |
| 04:53:47 | B | Swap | 160,768.83 USTR → UST1 → cUSTC → 382.66 CL8Y | `D2BE2D3C1BC5FF82C229FF8E266AA8DE5F82D2F124AAF56126A216AB8D171435` |
| 04:53:04 | B | Swap | 1,118.266797 UST1 → cUSTC → 1,018.67 CL8Y | `4898566E7A3D6CC3245A49E3D0040BC7D3979E149F58BDFC9D8596438407D578` |
| 04:48:39 | B | LP withdrawal | UST1/USTR pair `terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy`; 3.07 UST1 + 469.27 USTR returned | `9151D132145AF854941657BC2997DB954AC019CD84C787CC2DA485EF214E8F46` |
| 04:43:17 | B | **Bridge out** | 342.71 USDT → BSC `0x46b2…6f8d` (nonce 154) | `FDDED73075377ADA5C102C76E53C08FBAB331EA934699CC2D2C3B42C852BF1D7` |
| 04:43:11 | B | **Bridge out** | 10.18 vFDUSD → BSC `0x377a…0e5b` (nonce 153) | `3A5D25F06E769EE61E2E8AC271646A8E0F9F57828102B054F2FB49F635286BAA` |
| 04:19:38 | A | **Bridge out** | 237.37 USDT → BSC `0x46b2…6f8d` (nonce 152) | `E76AF7A330D76A60EC0557347724E2405B07CF1B4878140EAA722AD400A093E7` |

All swaps routed through router `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw`. A further swap from wallet A at 05:04:02 UTC (36.53 UST1, tx `27799A1B220324ACA0750818CA144E9E0203E907690799A3835B01E908F18618`) was logged by our arb bot as its own trade and is not counted here.

## What we found

The transactions were signed with our keys but not by our bot, and not from any machine we run. We are treating the wallet seed as compromised.

- **Not the bot.** None of the ten appear in the bot's own action log, and the bot's code has no bridge call at all.
- **Not our trading server.** No SSH login reached it during the 04:19–05:01 UTC window. Only routine scheduled jobs (monitoring and alerts) ran, and none of them signs transactions. No unknown process or recently changed script was found.
- **Not our monitoring server.** It holds no wallet key, and had no logins in the window.
- **Operator's PC: no sign of use so far.** When checked after the incident, no bot or unknown process was running and no suspicious startup entries were present. A full scan is still to do.

Open question: where the seed was exposed. We are auditing every place it has been stored.

## Reported actions

**Done (15 October, from 06:35 BST)**

- Stopped and disabled all trading bots on our server, so they cannot restart on reboot.
- Paused every scheduled job that touches the wallets, and the automated review agents.
- Traced all transactions from both wallets since 14 October 16:00 UTC (this report).
