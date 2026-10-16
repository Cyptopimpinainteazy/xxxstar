import { NextRequest, NextResponse } from 'next/server';
import {
  getNetworkStats,
  getRecentBlocks,
  getRecentExtrinsics,
  getBlock,
  getAccountInfo,
  getAuthorities,
  getAuthorizedAccounts,
  getCanonicalBalance,
  getBlockExtrinsics,
} from '@/lib/substrate';

// Real blockchain API endpoints
export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const type = searchParams.get('type');

    switch (type) {
      case 'stats': {
        const stats = await getNetworkStats();
        return NextResponse.json(stats);
      }

      case 'blocks': {
        const count = parseInt(searchParams.get('count') || '10');
        const blocks = await getRecentBlocks(count);
        return NextResponse.json(blocks);
      }

      case 'block': {
        const blockId = searchParams.get('id');
        if (!blockId) {
          return NextResponse.json({ error: 'Block ID required' }, { status: 400 });
        }
        const block = await getBlock(isNaN(Number(blockId)) ? blockId : Number(blockId));
        if (!block) {
          return NextResponse.json({ error: 'Block not found' }, { status: 404 });
        }
        return NextResponse.json(block);
      }

      case 'block-extrinsics': {
        const blockIdForExts = searchParams.get('blockId');
        if (!blockIdForExts) {
          return NextResponse.json({ error: 'Block ID required' }, { status: 400 });
        }
        const extrinsics = await getBlockExtrinsics(
          isNaN(Number(blockIdForExts)) ? blockIdForExts : Number(blockIdForExts)
        );
        return NextResponse.json(extrinsics);
      }

      case 'transactions':
      case 'extrinsics': {
        const count = parseInt(searchParams.get('count') || '20');
        const extrinsics = await getRecentExtrinsics(count);
        return NextResponse.json(extrinsics);
      }

      case 'account': {
        const address = searchParams.get('address');
        if (!address) {
          return NextResponse.json({ error: 'Address required' }, { status: 400 });
        }
        const account = await getAccountInfo(address);
        if (!account) {
          return NextResponse.json({ error: 'Account not found' }, { status: 404 });
        }
        return NextResponse.json(account);
      }

      case 'balance': {
        const account = searchParams.get('account');
        const assetId = parseInt(searchParams.get('assetId') || '0');
        if (!account) {
          return NextResponse.json({ error: 'Account required' }, { status: 400 });
        }
        const balance = await getCanonicalBalance(account, assetId);
        return NextResponse.json({ balance });
      }

      case 'authorities': {
        const authorities = await getAuthorities();
        return NextResponse.json(authorities);
      }

      case 'authorized-accounts': {
        const accounts = await getAuthorizedAccounts();
        return NextResponse.json(accounts);
      }

      default:
        return NextResponse.json({ error: 'Invalid type parameter' }, { status: 400 });
    }
  } catch (error) {
    console.error('API Error:', error);
    const message = error instanceof Error ? error.message : 'Failed to fetch data';
    return NextResponse.json({ error: message }, { status: 500 });
  }
}