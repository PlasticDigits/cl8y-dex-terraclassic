#!/usr/bin/env python3
"""Journal containment/restoration separately from the immutable rollout plan."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT/'scripts/lib'))
from route_tax_rollout import Rollout, digest, require, write_json


def aggregate_fees(state_dir, state):
    root = Path(state_dir)
    journal = json.loads((root/'journal.json').read_text())
    fees = sum(op.get('fee_uluna', 0) for op in journal['operations'].values())
    fees += sum(op.get('fee_uluna', 0) for op in state['operations'].values())
    for name in ('fund', 'swap'):
        p = root/'live-smoke'/(name+'-pending.json')
        if p.exists():
            fees += json.loads(p.read_text())['fee_uluna']
    require(isinstance(fees, int) and fees >= 0, 'Invalid aggregate fee accounting')
    return fees, journal['identity']['config']['max_total_fee_uluna']


def load(chain, state_dir):
    root = Path(state_dir)
    journal = json.loads((root/'journal.json').read_text())
    config = journal['identity']['config']
    require(chain.network() == config['chain_id'] == 'columbus-5', 'Containment chain mismatch')
    factory = chain.info(config['factory'])
    require(factory['admin'] == config['governance'], 'Containment factory custody changed')
    require(int(factory['code_id']) == journal['codes']['factory'], 'Containment factory code changed')
    require(chain.code_hash(int(factory['code_id'])) == journal['identity']['artifacts']['factory']['sha256'],
            'Containment factory checksum mismatch')
    targets = {p['address']: p['paused'] for p in journal['inventory']['pairs']
               if p['address'] in journal['inventory']['affected_pairs']}
    identity = {'manifest_identity': journal['identity_hash'], 'targets': targets,
                'factory': config['factory'], 'governance': config['governance']}
    path = root/'containment.json'
    if path.exists():
        state = json.loads(path.read_text())
        require(state['identity'] == identity and state['identity_hash'] == digest(identity), 'Containment identity changed')
    else:
        state = {'identity': identity, 'identity_hash': digest(identity), 'operations': {},
                 'introduced': [], 'external_pauses': [], 'status': 'new', 'round': 1}
    return path, state


def operation(chain, state_dir, path, state, name, action):
    entry = state['operations'].setdefault(name, {'action': action})
    require(entry['action'] == action, 'Containment checkpoint action changed')
    if entry.get('tx_hash'):
        chain.validate_saved(action, entry)
        receipt = chain.receipt(entry['tx_hash'])
        require(receipt is not None, f"Containment inclusion unknown for {entry['tx_hash']}; do not re-sign")
        Rollout._receipt_ok(receipt)
        require(receipt['txhash'] == entry['tx_hash'], 'Containment receipt/hash mismatch')
        entry['receipt'] = receipt
        write_json(path, state)
        return receipt
    write_json(path, state)
    def signed(tx_hash, signed_path, fee):
        total, cap = aggregate_fees(state_dir, state)
        require(total+fee <= cap, 'Rollout + smoke + containment aggregate fee cap exceeded')
        entry.update(tx_hash=tx_hash, signed_path=signed_path, fee_uluna=fee)
        write_json(path, state)
    receipt = chain.transaction(action, signed)
    Rollout._receipt_ok(receipt)
    entry['receipt'] = receipt
    write_json(path, state)
    return receipt


def contain(chain, state_dir, reason):
    path, state = load(chain, state_dir)
    if state['status'] == 'restored':
        state.update(round=state['round']+1, introduced=[], external_pauses=[])
    state.update(status='containing', reason=reason)
    write_json(path, state)
    for pair, originally_paused in state['identity']['targets'].items():
        if originally_paused:
            continue
        current = chain.smart(pair, {'is_paused': {}})['paused']
        name = f"round:{state['round']}:pause:{pair}"
        if name not in state['operations'] and current:
            if pair not in state['external_pauses']:
                state['external_pauses'].append(pair)
                write_json(path, state)
            continue
        if pair not in state['introduced']:
            state['introduced'].append(pair)
            write_json(path, state)
        action = {'op': 'execute', 'role': 'gov', 'contract': state['identity']['factory'],
                  'msg': {'set_pair_paused': {'pair': pair, 'paused': True}}}
        operation(chain, state_dir, path, state, name, action)
        require(chain.smart(pair, {'is_paused': {}})['paused'], 'Containment pause readback failed')
    state['status'] = 'contained'
    write_json(path, state)


def restore_after_success(chain, state_dir):
    path, state = load(chain, state_dir)
    evidence_path = Path(state_dir)/'live-smoke/evidence.json'
    require(evidence_path.exists(), 'No smoke evidence exists to authorize containment restoration')
    evidence = json.loads(evidence_path.read_text())
    require(evidence['status'] == 'PASS' and evidence['manifest_identity'] == state['identity']['manifest_identity'],
            'Contained markets require successful original smoke reconciliation before restoration')
    for receipt in evidence['receipts']:
        actual = chain.receipt(receipt['txhash'])
        require(actual is not None, 'Successful smoke receipt unavailable')
        Rollout._receipt_ok(actual)
    require(state['status'] in ('contained', 'restoring', 'restored'), 'Containment is incomplete; finish contained-state reconciliation first')
    for pair in state['introduced']:
        name = f"round:{state['round']}:pause:{pair}"
        pause_entry = state['operations'][name]
        operation(chain, state_dir, path, state, name, pause_entry['action'])
    state['status'] = 'restoring'
    write_json(path, state)
    for pair in state['introduced']:
        action = {'op': 'execute', 'role': 'gov', 'contract': state['identity']['factory'],
                  'msg': {'set_pair_paused': {'pair': pair, 'paused': False}}}
        operation(chain, state_dir, path, state, f"round:{state['round']}:restore:{pair}", action)
        require(not chain.smart(pair, {'is_paused': {}})['paused'], 'Containment restore readback failed')
    state['status'] = 'restored'
    write_json(path, state)
