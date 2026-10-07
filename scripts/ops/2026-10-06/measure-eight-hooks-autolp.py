#!/usr/bin/env python3
import argparse,base64,hashlib,importlib.util,json,pathlib,time
from datetime import datetime,timezone
ROOT=pathlib.Path(__file__).resolve().parents[3]
spec=importlib.util.spec_from_file_location('lt',ROOT/'scripts/qa/localterra-route-tax-1381.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
parser=argparse.ArgumentParser(description='Concrete eight burn hooks and AutoLP gas on an owned synthetic chain; no reset or production signing')
parser.add_argument('--container',default='cl8y-dex-functional1400-localterra-1')
parser.add_argument('--evidence-dir',type=pathlib.Path,default=pathlib.Path.home()/'.local/state/cl8y-dex/release-20261006-eight-hooks')
args=parser.parse_args();p=args.evidence_dir;p.mkdir(parents=True,exist_ok=True,mode=0o700);artifacts=ROOT/'smartcontracts/artifacts'
chain=m.LocalTerra(args.container,p/'max-hook-codes.json'); owner=chain.address('test1')
codes={n:chain.store(artifacts/(n+'.wasm')) for n in ['cw20_mintable','cl8y_dex_factory','cl8y_dex_pair','cl8y_dex_burn_hook','cl8y_community_tax_token','cl8y_community_tax_autolp']}
factory=chain.instantiate(codes['cl8y_dex_factory']['code_id'],{'governance':owner,'treasury':chain.address('test2'),'default_fee_bps':30,'pair_code_id':codes['cl8y_dex_pair']['code_id'],'lp_token_code_id':codes['cw20_mintable']['code_id'],'whitelisted_code_ids':[codes['cw20_mintable']['code_id'],codes['cl8y_community_tax_token']['code_id']],'pair_creation_fee_uluna':'0'},'max8-factory')
quote=chain.instantiate(codes['cw20_mintable']['code_id'],{'name':'Maximum hooks quote','symbol':'MAXQ','decimals':6,'initial_balances':[{'address':owner,'amount':'1000000000000000'}],'mint':None},'max8-quote')
token=chain.instantiate(codes['cl8y_community_tax_token']['code_id'],{'name':'MaximumHooksAutoLP','symbol':'MAXLP','decimals':6,'initial_balances':[{'address':owner,'amount':'1000000000000000'}],'manager':owner,'treasury':owner,'buy_bps':0,'sell_bps':0,'max_buy_bps':0,'max_sell_bps':0,'max_transfer_bps':0,'factory':factory,'router':None,'ust1':quote,'cmm_treasury':owner,'features':['auto_v2_lp'],'mint':None,'transfer_bps':None,'sinks':None,'autolp':None,'launcher':owner,'launch_guards':None},'max8-token')
infos=[m.asset(token),m.asset(quote)];chain.execute(factory,{'create_pair':{'asset_infos':infos}});pair=chain.smart(factory,{'pair':{'asset_infos':infos}})['pair']['contract_addr']
for a in [token,quote]:chain.execute(a,{'increase_allowance':{'spender':pair,'amount':'1000000000000'}})
chain.execute(pair,{'provide_liquidity':{'assets':[{'info':m.asset(token),'amount':'1000000000000'},{'info':m.asset(quote),'amount':'1000000000000'}],'slippage_tolerance':None}})
def swap(floor):
 msg={'swap':{'max_spread':'0.5','belief_price':None,'to':None,'min_return':floor}}
 return {'send':{'contract':pair,'amount':'1000000','msg':base64.b64encode(json.dumps(msg).encode()).decode()}}
baseline=chain.execute(token,swap('1'))
hooks=[]
for i in range(8):
 h=chain.instantiate(codes['cl8y_dex_burn_hook']['code_id'],{'burn_token':quote,'burn_percentage_bps':1,'admin':owner},'max8-hook'+str(i));chain.execute(h,{'update_allowed_pairs':{'add':[pair],'remove':[]}});hooks.append(h)
chain.execute(factory,{'set_pair_hooks':{'pair':pair,'hooks':hooks}})
quote_result=chain.smart(pair,{'hybrid_simulation':{'offer_asset':{'info':m.asset(token),'amount':'1000000'},'hybrid':None,'greedy':None,'trader':owner,'sender':owner,'belief_price':None}})
net=int(quote_result['return_amount']);assert net>0
snapshot=lambda: {'pool':chain.smart(pair,{'pool':{}}),'token_owner':chain.balance(token,owner),'quote_owner':chain.balance(quote,owner),'quote_supply':chain.smart(quote,{'token_info':{}})['total_supply']}
before=snapshot();missing=chain.execute(token,swap(None),expect_failure=True);assert snapshot()==before
before_balance=chain.balance(quote,owner);maximum=chain.execute(token,swap(str(net)));delivered=chain.balance(quote,owner)-before_balance;assert delivered==net,(net,delivered)
lp=chain.smart(pair,{'pair':{}})['liquidity_token']
sister=chain.instantiate(codes['cl8y_community_tax_autolp']['code_id'],{'token':token,'manager':owner,'factory':factory,'router':None,'pair':pair,'quote_token':quote,'threshold':'1000','lp_recipient':owner,'skim_max_spread':'0.01','skim_min_return':None},'max8-autolp')
chain.execute(token,{'bind_autolp':{'autolp':sister}})
policy=chain.smart(sister,{'get_lp_policy':{}});assert policy and int(policy['tax_reserve'])>0
chain.execute(token,{'transfer':{'recipient':sister,'amount':'1000000'}});lp_before=chain.balance(lp,owner);skim=chain.execute(sister,{'skim_to_lp':{}});minted=chain.balance(lp,owner)-lp_before;assert minted>0;assert chain.smart(sister,{'get_lp_policy':{}})==policy
record={'utc':datetime.now(timezone.utc).isoformat(),'status':'PASS','scope':'synthetic LocalTerra concrete eight burn hooks at1bps; not arbitrary-hook gas envelope or production provider qualification','codes':codes,'factory':factory,'pair':pair,'hooks':hooks,'autolp':sister,'baseline_swap':{k:baseline[k] for k in ['txhash','gas_used']},'eight_hook_swap':{k:maximum[k] for k in ['txhash','gas_used']},'missing_floor_rollback':{k:missing[k] for k in ['txhash','gas_used','raw_log']},'autolp_eight_hook_skim':{k:skim[k] for k in ['txhash','gas_used']},'quote_net':str(net),'delivered_net':str(delivered),'minted_lp':str(minted),'policy_preserved':True,'receipts':chain.receipts}
(p/'max-hooks-autolp.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:record[k] for k in ['status','baseline_swap','eight_hook_swap','autolp_eight_hook_skim','quote_net','minted_lp']}))
