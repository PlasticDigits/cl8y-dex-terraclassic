import importlib.util,json,pathlib,tempfile,types,unittest
spec=importlib.util.spec_from_file_location('debt',pathlib.Path(__file__).with_name('bridge-debt.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Fake:
 def __init__(self):self.args=types.SimpleNamespace(max_total_fee_uluna=2000000000);self.outstanding=100;self.cash=0;self.receipts={};self.calls=[]
 def validate_saved(self,a,e):pass
 def receipt(self,h):return self.receipts.get(h)
 def transaction(self,a,signed):
  h=str(len(self.calls)+1).zfill(64);signed(h,'/private/'+h,100);self.calls.append(a)
  if 'send' in a['msg']:self.outstanding-=1
  elif 'transfer' in a['msg']:self.cash+=1
  else:self.cash-=1
  r={'txhash':h,'code':0};self.receipts[h]=r;return r
class ProbeOps(m.DebtOps):
 def bindings(self):pass
 def artifacts(self):pass
 def snapshot(self):return {'market':{'outstanding':str(self.c.outstanding)},'supply':str(self.c.outstanding)}
 def verify(self):
  self.reconcile();expected=100-int(bool(self.s['operations'].get('repay-one-unit',{}).get('receipt')));assert self.c.outstanding==expected
class Tests(unittest.TestCase):
 def make(self,p,c):
  ops=ProbeOps(c,p);identity={'test':'explicit approved mock'};ops.s={'identity':identity,'identity_hash':m.hashlib.sha256(json.dumps(identity,sort_keys=True).encode()).hexdigest(),'operations':{}};m.write_json(pathlib.Path(p)/'manifest.json',identity);ops.save();return ops
 def test_idempotent_probes_and_accounting_reassertion(self):
  with tempfile.TemporaryDirectory() as p:
   c=Fake();o=self.make(p,c);o.probes();o.probes();self.assertEqual(len(c.calls),3);self.assertEqual((c.outstanding,c.cash),(99,0));c.outstanding-=1
   with self.assertRaises(AssertionError):o.probes()
 def test_pending_receipt_reconciles_without_new_burn(self):
  with tempfile.TemporaryDirectory() as p:
   c=Fake();o=self.make(p,c);o.probes();del o.s['operations']['repay-one-unit']['receipt'];o.save();o.probes();self.assertEqual(len(c.calls),3);self.assertEqual(c.outstanding,99)
unittest.main()
