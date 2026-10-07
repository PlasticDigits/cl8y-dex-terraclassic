#!/usr/bin/env python3
"""Reviewed Bridge-Debt production rollout using the existing durable signing transport."""
from __future__ import annotations
import argparse,fcntl,hashlib,json,os,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'scripts/lib'))
from route_tax_rollout_cli import TerradChain,parser as transport_parser
from route_tax_rollout import Rollout,RolloutError,require,write_json
ISSUER='terra12c6hyez8fvvcrfhvq04xc8t63y7kmmdrkktlwefx4txrgf6peuxqk5x6rq'
DEBT='terra1qrxch5xafa077952ds0gep8xm4rq4fuwa30xwc2m3flm4a4swz4qh42xm7'
USDT='terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4'
CMM='terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2'
GOV='terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7'
ADMIN='terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l'
BRIDGE='terra18m02l2f43c2dagqnz3kfccpgz9pzzz5hk9l5mh5wvr6dcvv47zfqdfs7la'
LIMITS={k:'10000000000000000000000' for k in ('max_per_issue','max_outstanding','max_per_epoch')};LIMITS['epoch_seconds']=86400

class DebtOps:
 def __init__(self,chain,state_dir,rebroadcast=False):
  self.c=chain;self.folder=Path(state_dir);self.path=self.folder/'journal.json';self.rebroadcast=rebroadcast
  self.hashes={item['filename']:item['sha256'] for item in json.loads(Path(__file__).with_name('bridge-debt-hashes.json').read_text())}
  self.s=json.loads(self.path.read_text()) if self.path.exists() else {}
 def save(self):write_json(self.path,self.s)
 def reconcile(self):
  for name,entry in self.s.get('operations',{}).items():
   if entry.get('tx_hash'):
    self.c.validate_saved(entry['action'],entry);r=self.c.receipt(entry['tx_hash'])
    if not r:
     require(self.rebroadcast,'Pending inclusion for '+name+'; reconcile without re-signing')
     r=self.c.rebroadcast(entry['signed_path'],entry['tx_hash'])
    Rollout._receipt_ok(r);entry['receipt']=r;self.save()
 def snapshot(self):
  return {'market':self.c.smart(ISSUER,{'market':{'underlying':USDT}}),'supply':self.c.smart(DEBT,{'token_info':{}})['total_supply'],'debtor_balance':self.c.smart(DEBT,{'balance':{'address':CMM}})['balance']}
 def artifacts(self):
  for name,digest in self.hashes.items():require(hashlib.sha256((ROOT/'smartcontracts/artifacts'/name).read_bytes()).hexdigest()==digest,'Reviewed Bridge-Debt artifact changed: '+name)
 def bindings(self):
  require(self.c.network()=='columbus-5','Chain identity mismatch')
  self.artifacts()
  for a,n,legacy in [(ISSUER,'bridge_debt.wasm',11709),(DEBT,'bridge_debt_token.wasm',11708)]:
   info=self.c.info(a);require(info['admin']==GOV,'Wasm admin changed')
   code=int(info['code_id']);target=self.s.get('codes',{}).get(n)
   require(code in {legacy,target},'Unexpected Bridge-Debt implementation')
   version=self.c.version(a);require(version=={'contract':'bridge-debt' if a==ISSUER else 'bridge-debt-token','version':'1.1.0' if target==code else '1.0.0'},'Unexpected Bridge-Debt cw2 identity')
   if code==target:require(self.c.code_hash(code)==self.hashes[n],'Migrated bytecode hash changed')
   elif self.s:require(self.c.code_hash(code)==self.s['baseline']['legacy_hashes'][str(code)],'Legacy bytecode hash changed')
  require(int(self.c.info(USDT)['code_id'])==10184,'USDT implementation changed')
  require(self.c.smart(USDT,{'minter':{}})=={'minter':GOV,'cap':None},'Primary minter changed')
  minters=self.c.smart(USDT,{'minters':{'limit':30}})['minters'];require(sorted(minters)==sorted([ISSUER,BRIDGE]),'Additional minters changed')
  cfg=self.c.smart(ISSUER,{'config':{}});require(cfg['owner']==GOV and cfg.get('pending_owner') is None,'Issuer owner changed')
 def plan(self):
  self.bindings()
  if self.s:return
  markets=self.c.smart(ISSUER,{'markets':{'limit':30}})['markets'];require(len(markets)==1 and markets[0]['underlying']==USDT and markets[0]['debt_token']==DEBT,'Unexpected market inventory')
  require(markets[0]['debtor']==CMM and markets[0]['decimals']==18,'Market units/debtor changed')
  supply=self.c.smart(DEBT,{'token_info':{}})['total_supply'];debtor=self.c.smart(DEBT,{'balance':{'address':CMM}})['balance']
  require(supply==debtor==markets[0]['outstanding']=='10000000000000000000000','Unexpected initial debt accounting')
  require(self.c.smart(DEBT,{'minter':{}})=={'minter':ISSUER,'cap':None},'Receipt minter changed')
  identity={'schema':1,'chain_id':'columbus-5','issuer':ISSUER,'receipt':DEBT,'underlying':USDT,'governance':GOV,'artifact_hashes':self.hashes,'limits':LIMITS,'paused':True,'max_total_fee_uluna':self.c.args.max_total_fee_uluna}
  self.s={'identity':identity,'identity_hash':hashlib.sha256(json.dumps(identity,sort_keys=True).encode()).hexdigest(),'baseline':{'market':markets[0],'supply':supply,'debtor_balance':debtor,'legacy_hashes':{str(i):self.c.code_hash(i) for i in (11708,11709)}},'operations':{},'codes':{},'status':'planned'}
  write_json(self.folder/'manifest.json',identity);self.save()
 def operation(self,name,action):
  require(json.loads((self.folder/'manifest.json').read_text())==self.s['identity'] and hashlib.sha256(json.dumps(self.s['identity'],sort_keys=True).encode()).hexdigest()==self.s['identity_hash'],'Immutable manifest changed')
  self.bindings();ops=self.s['operations'];entry=ops.setdefault(name,{'action':action});require(entry['action']==action,'Saved operation changed');self.save()
  if entry.get('tx_hash'):
   self.c.validate_saved(action,entry);r=self.c.receipt(entry['tx_hash'])
   if not r:
    require(self.rebroadcast,'Saved transaction inclusion unknown; reconcile or use --rebroadcast-pending for identical signed bytes')
    r=self.c.rebroadcast(entry['signed_path'],entry['tx_hash'])
   Rollout._receipt_ok(r);entry['receipt']=r;self.save();return r
  def signed(tx_hash,path,fee):
   require(sum(int(e.get('fee_uluna',0)) for e in ops.values())+fee<=self.c.args.max_total_fee_uluna,'Aggregate fee budget exceeded')
   entry.update(tx_hash=tx_hash,signed_path=path,fee_uluna=fee);self.save()
  r=self.c.transaction(action,signed);Rollout._receipt_ok(r);require(r['txhash']==entry['tx_hash'],'Receipt differs');entry['receipt']=r;self.save();return r
 def store(self,name):
  if name not in self.s['codes']:
   r=self.operation('store:'+name,{'op':'store','role':'store','path':str(ROOT/'smartcontracts/artifacts'/name)});self.s['codes'][name]=int(r['code_id']);self.save()
  require(self.c.code_hash(self.s['codes'][name])==self.hashes[name],'Stored hash differs');return self.s['codes'][name]
 def execute(self):
  self.plan();self.reconcile();debtcode=self.store('bridge_debt_token.wasm');issuercode=self.store('bridge_debt.wasm')
  for address,code,msg,name in [(DEBT,debtcode,{},'receipt'),(ISSUER,issuercode,{'legacy_pins':[{'underlying':USDT,'pins':{'debt_code_id':debtcode,'underlying_code_id':10184,'multi_minter':True,'minter':GOV,'minter_cap':None,'additional_minters':sorted([ISSUER,BRIDGE])}}]},'issuer')]:
   if int(self.c.info(address)['code_id'])!=code:self.operation('migrate:'+name,{'op':'migrate','role':'gov','contract':address,'code_id':code,'msg':msg})
  market=self.snapshot()['market']
  if market.get('limits')!=LIMITS or market['paused'] is not True:self.operation('finite-paused-limits',{'op':'execute','role':'gov','contract':ISSUER,'msg':{'configure_market':{'underlying':USDT,'limits':LIMITS,'paused':True}}})
  if self.c.smart(ISSUER,{'config':{}})['debt_code_id']!=debtcode:self.operation('receipt-default',{'op':'execute','role':'gov','contract':ISSUER,'msg':{'set_debt_code_id':{'code_id':debtcode}}})
  self.verify();self.s['status']='migrated';self.save()
 def verify(self):
  self.reconcile();self.bindings();snap=self.snapshot();market=snap['market'];expected=int(self.s['baseline']['supply'])-(1 if 'repay-one-unit' in self.s['operations'] and self.s['operations']['repay-one-unit'].get('receipt') else 0)
  require(int(snap['supply'])==int(snap['debtor_balance'])==int(market['outstanding'])==expected,'Debt accounting changed unexpectedly')
  require(market.get('limits')==LIMITS and market['paused'] is True and market.get('pending_upgrade') is None,'Limits/pause/upgrade differ')
  require(market['pins']=={'debt_code_id':self.s['codes']['bridge_debt_token.wasm'],'underlying_code_id':10184,'multi_minter':True,'minter':GOV,'minter_cap':None,'additional_minters':sorted([ISSUER,BRIDGE])},'Reviewed market pins differ')
  for name,entry in self.s['operations'].items():
   if entry.get('tx_hash'):
    self.c.validate_saved(entry['action'],entry);receipt=self.c.receipt(entry['tx_hash']);require(receipt is not None,'Saved operation inclusion unknown: '+name);Rollout._receipt_ok(receipt)
  write_json(self.folder/'readback.json',snap);return snap
 def probes(self):
  self.verify()
  repay={'op':'execute','role':'cl8y2_admin','contract':USDT,'msg':{'send':{'contract':ISSUER,'amount':'1','msg':'eyJyZXBheSI6e319'}}}
  self.operation('repay-one-unit',repay);self.verify()
  self.operation('plain-transfer-one-unit',{'op':'execute','role':'cl8y2_admin','contract':USDT,'msg':{'transfer':{'recipient':ISSUER,'amount':'1'}}})
  before=self.snapshot();self.operation('recover-one-unit',{'op':'execute','role':'gov','contract':ISSUER,'msg':{'recover':{'underlying':USDT,'recipient':ADMIN,'amount':'1'}}});self.verify();after=self.snapshot()
  require(before['supply']==after['supply'] and before['market']['outstanding']==after['market']['outstanding'],'Recover changed debt')
  self.s['status']='complete';self.save()

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['plan','execute','verify','probes']);p.add_argument('--state-dir',default=str(Path.home()/'.local/state/cl8y-dex/release-20261006-bridge-debt'));p.add_argument('--rebroadcast-pending',action='store_true');a=p.parse_args()
 args=transport_parser().parse_args(['--state-dir',a.state_dir,'--admin-key','cl8y2_admin','--max-gas','25000000','--max-total-fee-uluna','2000000000']);args.governance=GOV
 args.read_node=args.read_node or 'https://rpc.terra-classic.hexxagon.io:443';args.execution_node=args.execution_node or 'https://rpc.terra-classic.hexxagon.io:443'
 folder=Path(a.state_dir);folder.mkdir(parents=True,exist_ok=True,mode=0o700);folder.chmod(0o700)
 with (folder/'lock').open('w') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);chain=TerradChain(args);ops=DebtOps(chain,a.state_dir,a.rebroadcast_pending)
  try:
   getattr(ops,{'probes':'probes'}.get(a.mode,a.mode))();print('PASS '+a.mode+'; journal '+str(ops.path))
  finally:chain.password=''
if __name__=='__main__':
 try:main()
 except (RolloutError,BlockingIOError) as e:sys.exit('FAIL: '+str(e))
