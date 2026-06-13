import { useState } from 'react';
export function useClashLog() {
  return useState({
    enable: true,
    logLevel: 'info',
    logFilter: 'all',
    logOrder: 'asc',
  });
}
