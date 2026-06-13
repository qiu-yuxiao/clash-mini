import { useState } from 'react';

const queryDataStore = new Map();

export const queryClient = {
  getQueryData: (key) => queryDataStore.get(key[0]),
  setQueryData: (key, value) => {
    queryDataStore.set(key[0], value);
  },
  removeQueries: (options) => {
    if (options && options.queryKey) {
      queryDataStore.delete(options.queryKey[0]);
    }
  }
};

export function useQueryClient() {
  return queryClient;
}

export function useQuery(options) {
  const queryKey = options.queryKey[0];
  const data = queryClient.getQueryData([queryKey]) ?? (options.initialData ? options.initialData() : undefined);
  return { data };
}
