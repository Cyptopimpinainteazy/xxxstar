import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FundingProposalEditor } from './FundingProposalEditor';
afterEach(() => vi.unstubAllGlobals());
const draft = {project_title:'Fixture project',research_question:'Fixture question',technical_approach:'Reference comparison',evaluation_plan:'Measured benchmark',organization_facts:'Unverified fixture organization',requested_usd_cents:null};
const source = {snapshot_id:'source-fixture',content_sha256:'a'.repeat(64),official_url:'https://www.grants.gov/search-results-detail/123'};
function respond(status=200) {
  const calls=vi.fn(async (_url:string, options?:RequestInit) => new Response(JSON.stringify(options?.method === 'POST'
    ? {version:1,source_snapshot_id:source.snapshot_id,package_sha256:'b'.repeat(64),package:{operator_self_report:draft},created_at:'2026-10-18T00:00:00Z'}
    : {source,versions:[],history_limit:25}),{status:options?.method === 'POST'?status:200}));
  vi.stubGlobal('fetch',calls);return calls;
}
async function fill() {
  await screen.findByLabelText('Project title');
  for(const [label,value] of [['Project title',draft.project_title],['Research question',draft.research_question],['Technical approach',draft.technical_approach],['Evaluation plan',draft.evaluation_plan],['Organization facts and supporting evidence',draft.organization_facts]]) {
    fireEvent.change(screen.getByLabelText(label),{target:{value}});
  }
}
describe('source-bound proposal packages',()=>{
  it('saves actual edited fields with optimistic source and version bindings',async()=>{
    const calls=respond();render(<FundingProposalEditor grantId="grant-fixture" token="fixture-credential"/>);await fill();
    fireEvent.click(screen.getByRole('button',{name:'Save new version'}));
    await screen.findByText('Version 1 saved. No application was submitted.');
    const [,options]=calls.mock.calls.find(([,options])=>options?.method==='POST')!;
    expect(JSON.parse(String(options?.body))).toEqual({...draft,expected_version:0,source_snapshot_id:source.snapshot_id});
    expect(screen.getByRole('button',{name:'Download version 1'})).toBeInTheDocument();
  });
  it('preserves edited text and exposes source/version conflict without invented success',async()=>{
    respond(400);render(<FundingProposalEditor grantId="grant-fixture" token="fixture-credential"/>);await fill();
    fireEvent.click(screen.getByRole('button',{name:'Save new version'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('Version was not saved');
    expect(screen.getByLabelText('Project title')).toHaveValue(draft.project_title);
    expect(screen.queryByText(/Version 1 saved/)).not.toBeInTheDocument();
  });
  it('rejects unsafe financial precision before sending a save',async()=>{
    const calls=respond();render(<FundingProposalEditor grantId="grant-fixture" token="fixture-credential"/>);await fill();
    fireEvent.change(screen.getByLabelText(/Requested amount/),{target:{value:'9007199254740992'}});
    fireEvent.click(screen.getByRole('button',{name:'Save new version'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('safe integer');
    expect(calls.mock.calls.some(([,options])=>options?.method==='POST')).toBe(false);
  });
  it('hides private form content when the operator credential changes',async()=>{
    respond();const {rerender}=render(<FundingProposalEditor grantId="grant-fixture" token="fixture-credential"/>);await fill();
    vi.stubGlobal('fetch',vi.fn(()=>new Promise(()=>{})));
    rerender(<FundingProposalEditor grantId="grant-fixture" token="different-fixture-credential"/>);
    await waitFor(()=>expect(screen.queryByLabelText('Project title')).not.toBeInTheDocument());
  });
});
