import pathlib,sys,tempfile,unittest,datetime as dt
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]))
import grantops
class TestGrantOps(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.db=pathlib.Path(self.tmp.name)/'test.db'
  self.lead={'id':'example','name':'Example','official_url':'https://example.org/grant','category':'gpu','entity_required':True}
 def tearDown(self):self.tmp.cleanup()
 def test_import_and_company_gate(self):
  grantops.import_leads([self.lead],self.db)
  self.assertEqual(grantops.pipeline(path=self.db)[0]['score'],0)
  self.assertEqual(grantops.pipeline('incorporated',self.db)[0]['score'],85)
 def test_bad_url(self):
  with self.assertRaises(ValueError):grantops.import_leads([{**self.lead,'official_url':'http://example.org'}],self.db)
 def test_past_deadline(self):
  grantops.import_leads([{**self.lead,'deadline':'2020-01-01'}],self.db)
  self.assertEqual(grantops.pipeline('incorporated',self.db)[0]['score'],0)
 def test_requires_human_approval(self):
  grantops.import_leads([self.lead],self.db)
  with self.assertRaises(ValueError):grantops.move('example','submitted',path=self.db)
  self.assertTrue(grantops.move('example','submitted','manual-approval-001',self.db)['no_external_action'])
 def test_update_preserves_stage(self):
  grantops.import_leads([self.lead],self.db)
  grantops.move('example','researching',path=self.db)
  grantops.import_leads([{**self.lead,'name':'New name'}],self.db)
  self.assertEqual(grantops.pipeline(path=self.db)[0]['stage'],'researching')
if __name__=='__main__':unittest.main()
