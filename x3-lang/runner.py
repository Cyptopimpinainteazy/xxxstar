#!/usr/bin/env python3
import argparse
import json
import os
import sys
import importlib.util


def load_module(path):
    path = os.fspath(path)
    spec = importlib.util.spec_from_file_location(os.path.splitext(os.path.basename(path))[0], path)
    module = importlib.util.module_from_spec(spec)
    sys.path.insert(0, os.path.dirname(path))
    try:
        spec.loader.exec_module(module)
    finally:
        sys.path.pop(0)
    return module


def validate_schema(output, schema_path):
    try:
        import jsonschema
    except ImportError:
        print('warning: jsonschema not installed, skipping schema validation', file=sys.stderr)
        return True

    with open(schema_path, 'r') as f:
        schema = json.load(f)

    jsonschema.validate(instance=output, schema=schema)
    return True


def parse_constraints(plan):
    constraints = plan.get('constraints', {})
    return {
        'min_profit': constraints.get('min_profit'),
        'max_slippage': constraints.get('max_slippage'),
        'timeout': constraints.get('timeout'),
        'atomic': constraints.get('atomic', False)
    }


def evaluate_constraints(plan):
    results = []
    constraints = parse_constraints(plan)
    estimates = plan.get('estimates', {})
    expected_profit = estimates.get('expected_profit_usd')
    slippage_usd = estimates.get('estimated_slippage_usd')
    start_amount = None
    fr = plan.get('from', {})
    amt = fr.get('amount')
    try:
        if amt is not None:
            start_amount = float(amt)
    except Exception:
        start_amount = None

    if constraints['min_profit'] is not None and expected_profit is not None:
        try:
            min_profit = float(str(constraints['min_profit']).split()[0])
            if expected_profit < min_profit:
                results.append({
                    'constraint': 'min_profit',
                    'ok': False,
                    'expected_profit_usd': expected_profit,
                    'min_required_profit_usd': min_profit
                })
            else:
                results.append({
                    'constraint': 'min_profit',
                    'ok': True,
                    'expected_profit_usd': expected_profit,
                    'min_required_profit_usd': min_profit
                })
        except Exception:
            results.append({'constraint': 'min_profit', 'ok': False, 'error': 'unparseable min_profit'})

    if constraints['max_slippage'] is not None and start_amount is not None:
        try:
            s = str(constraints['max_slippage']).strip()
            if s.endswith('%'):
                max_slippage_pct = float(s[:-1]) / 100.0
            else:
                max_slippage_pct = float(s)
            actual_pct = slippage_usd / start_amount if start_amount else None
            if actual_pct is not None and actual_pct > max_slippage_pct:
                results.append({
                    'constraint': 'max_slippage',
                    'ok': False,
                    'actual_slippage_pct': actual_pct,
                    'max_slippage_pct': max_slippage_pct
                })
            else:
                results.append({
                    'constraint': 'max_slippage',
                    'ok': True,
                    'actual_slippage_pct': actual_pct,
                    'max_slippage_pct': max_slippage_pct
                })
        except Exception:
            results.append({'constraint': 'max_slippage', 'ok': False, 'error': 'unparseable max_slippage'})

    if constraints['atomic']:
        results.append({'constraint': 'atomic', 'ok': True, 'note': 'atomic intent requested'})

    return results


def run(input_path, no_schema=False, mock_rpc=False, dry_run=False, proof_bundle=None):
    root = os.path.dirname(__file__)
    cli = load_module(os.path.join(root, 'cli.py'))
    typechecker = load_module(os.path.join(root, 'typechecker.py'))
    planner = load_module(os.path.join(root, 'planner.py'))
    simulator = load_module(os.path.join(root, 'simulator.py'))
    emitter = load_module(os.path.join(root, 'emitter', '__init__.py'))
    mock_rpc_module = None
    if mock_rpc or dry_run:
        dry_run = True
        mock_rpc_module = load_module(os.path.join(root, 'mock_rpc.py'))

    intent = cli.parse_file(input_path)
    if not no_schema:
        validate_schema(intent, os.path.join(root, 'schema.json'))

    valid, errors = typechecker.typecheck(intent)
    if not valid:
        return {'status': 'error', 'errors': typechecker.errors_to_json(errors)}

    plan = planner.plan(intent)
    plan = simulator.simulate(plan)

    # Two-phase commit simulation: prepare (emit payloads), then simulate execution
    emitted = emitter.emit(plan, dry_run=dry_run, proof_bundle=proof_bundle)
    plan['emitted'] = emitted

    # Simulate execution of emitted steps; if any simulated failure occurs, produce rollback payloads
    execution = []
    failed = False
    rollback_payloads = []
    if mock_rpc_module:
        execution = mock_rpc_module.execute_dry_run(emitted.get('emitted', []))
        failed = any(not e.get('ok', False) for e in execution)
        for e in execution:
            if not e.get('ok', False) and 'reason' not in e:
                e['reason'] = 'mock rpc failure'
    else:
        for out in emitted.get('emitted', []):
            # simple rule: if emitted contains 'error' field, mark failure
            if out.get('error'):
                execution.append({'ok': False, 'reason': out.get('error'), 'step': out})
                failed = True
                break
            execution.append({'ok': False, 'reason': 'production backend not configured', 'code': 'X3_BACKEND_REQUIRED', 'step': out}); failed = True; break

    if failed:
        # produce rollback payloads: naive inverse of emitted steps
        for e in emitted.get('emitted', [])[::-1]:
            if e.get('type') == 'swap':
                rollback_payloads.append({
                    'type': 'swap',
                    'chain': e.get('chain'),
                    'payload': f"rollback-swap-{e.get('from')}-to-{e.get('to')}"
                })
            elif e.get('type') == 'bridge':
                rollback_payloads.append({
                    'type': 'bridge',
                    'chain': e.get('chain'),
                    'payload': f"rollback-bridge-{e.get('asset')}-{e.get('transaction_id')}"
                })
            else:
                rollback_payloads.append({'type': 'unknown', 'payload': 'noop'})
        plan['execution'] = execution
        plan['rollback'] = rollback_payloads
        plan['status'] = 'rolled_back'
        plan['constraint_results'] = evaluate_constraints(plan)
        return plan

    plan['execution'] = execution
    plan['constraint_results'] = evaluate_constraints(plan)
    plan['status'] = 'ok'
    return plan


def main():
    parser = argparse.ArgumentParser(description='Run X3 Lang intent through parsing, typechecking, planning, and simulation')
    parser.add_argument('input', help='input .x3 file')
    parser.add_argument('--no-schema', action='store_true', help='skip JSON schema validation')
    parser.add_argument('--mock-rpc', action='store_true', help='deprecated alias for --dry-run')
    parser.add_argument('--dry-run', action='store_true', help='execute with explicit dry-run adapter; never reports production settlement')
    args = parser.parse_args()

    result = run(args.input, no_schema=args.no_schema, mock_rpc=args.mock_rpc, dry_run=args.dry_run)
    print(json.dumps(result, indent=2))
    if result.get('status') == 'error':
        sys.exit(1)


if __name__ == '__main__':
    main()
