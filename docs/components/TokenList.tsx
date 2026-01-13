import { useQuery } from '@tanstack/react-query'

export function TokenListDemo() {
  const tokenList = useQuery({
    queryKey: ['tokenList', 42431],
    queryFn: async () => {
      const response = await fetch('https://tokenlist.tempo.xyz/list/42431')
      const data = await response.json()
      if (!Object.hasOwn(data, 'tokens')) throw new Error('Invalid token list')
      return data.tokens as Array<{
        name: string
        symbol: string
        decimals: number
        chainId: number
        address: string
        logoURI: string
        extensions: { chain: string }
      }>
    },
  })

  return (
    <ul className="list-none gap-3 flex flex-col justify-center">
      {tokenList.data?.map((token) => (
        <li key={token.address} title={token.address}>
          <a
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-content"
            href={`https://tokenlist.tempo.xyz/asset/42431/${token.address}`}
          >
            <img src={token.logoURI} alt={token.name} className="size-7.5" />
            <span className="text-xl font-medium tracking-wider">
              {token.name}
            </span>
          </a>
        </li>
      ))}
    </ul>
  )
}
