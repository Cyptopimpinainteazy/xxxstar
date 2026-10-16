/**
 * Explorer App Providers
 * 
 * Wraps the explorer app with necessary providers for chain connectivity.
 */

'use client';

import { ReactNode, useState, useEffect } from 'react';
import { SWRConfig } from 'swr';
import { ChainProvider } from '@atlas-sphere/shared';

interface ProvidersProps {
  children: ReactNode;
}

export function Providers({ children }: ProvidersProps) {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  // Prevent hydration mismatch
  if (!mounted) {
    return (
      <SWRConfig
        value={{
          revalidateOnFocus: false,
          dedupingInterval: 2000,
        }}
      >
        {children}
      </SWRConfig>
    );
  }

  return (
    <SWRConfig
      value={{
        revalidateOnFocus: false,
        dedupingInterval: 2000,
        errorRetryCount: 3,
        errorRetryInterval: 5000,
      }}
    >
      <ChainProvider autoConnect>
        {children}
      </ChainProvider>
    </SWRConfig>
  );
}

export default Providers;
