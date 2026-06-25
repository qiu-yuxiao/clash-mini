import { QueryClient } from '@tanstack/react-query'

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: Infinity,
      retry: 3,
      retryDelay: 5000,
      refetchOnWindowFocus: false,
    },
  },
})
