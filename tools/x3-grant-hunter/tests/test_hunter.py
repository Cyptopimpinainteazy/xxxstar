import unittest,sys,pathlib,tempfile
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]))
import hunter
class Tests(unittest.TestCase):
 def test_company_gate(self):self.assertEqual(hunter.qualify(hunter.SOURCES[0],'forming'),'blocked_pending_incorporation')
 def test_free_eligibility(self):self.assertEqual(hunter.qualify(hunter.SOURCES[1],'forming'),'candidate_needs_review')
 def test_paid_rejected(self):
  with self.assertRaises(ValueError):hunter.allocate('test',estimated_cost=0.01)
 def test_unknown_provider_rejected(self):
  with self.assertRaises(ValueError):hunter.allocate('test',provider='other')
if __name__=='__main__':unittest.main()
