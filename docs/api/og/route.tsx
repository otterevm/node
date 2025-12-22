import { ImageResponse } from '@vercel/og'
import { OgImageTemplate } from '../../components/OgImageTemplate'

export const runtime = 'edge'

// Font URLs - matching explorer PR #279 setup
// @vercel/og requires TTF/OTF format (not WOFF2)
// GeistMono: Using TTF from unpkg (same CDN as PR #279, but TTF instead of WOFF2)
// Inter: Using TTF from GitHub releases (official source, reliable)
const FONT_MONO_URL =
  'https://unpkg.com/geist/dist/fonts/geist-mono/GeistMono-Regular.ttf'
const FONT_INTER_URL =
  'https://github.com/rsms/inter/releases/download/v3.19/Inter-Medium.ttf'

// Cache fonts in memory
let fontCache: { mono: ArrayBuffer; inter: ArrayBuffer } | null = null
let fontsInFlight: Promise<{ mono: ArrayBuffer; inter: ArrayBuffer }> | null =
  null

async function loadFonts() {
  if (fontCache) return fontCache
  if (!fontsInFlight) {
    fontsInFlight = Promise.all([
      fetch(FONT_MONO_URL)
        .then((r) => {
          if (!r.ok) {
            console.error(`Failed to fetch GeistMono: ${r.status} ${r.statusText}`)
            throw new Error(`Font fetch failed: ${r.status}`)
          }
          const contentType = r.headers.get('content-type') || ''
          console.log(`GeistMono content-type: ${contentType}`)
          return r.arrayBuffer()
        }),
      fetch(FONT_INTER_URL)
        .then((r) => {
          if (!r.ok) {
            console.error(`Failed to fetch Inter: ${r.status} ${r.statusText}`)
            throw new Error(`Font fetch failed: ${r.status}`)
          }
          const contentType = r.headers.get('content-type') || ''
          console.log(`Inter content-type: ${contentType}`)
          return r.arrayBuffer()
        }),
    ])
      .then(([mono, inter]) => {
        // Check file signatures to verify they're TTF
        const monoView = new Uint8Array(mono.slice(0, 4))
        const interView = new Uint8Array(inter.slice(0, 4))
        const monoSig = Array.from(monoView)
          .map((b) => b.toString(16).padStart(2, '0'))
          .join('')
        const interSig = Array.from(interView)
          .map((b) => b.toString(16).padStart(2, '0'))
          .join('')
        console.log(`GeistMono signature: ${monoSig} (should be 00010000 for TTF)`)
        console.log(`Inter signature: ${interSig} (should be 00010000 for TTF)`)

        fontCache = { mono, inter }
        fontsInFlight = null
        return fontCache
      })
      .catch((error) => {
        console.error('Font loading error:', error)
        fontsInFlight = null
        throw error
      })
  }
  return fontsInFlight
}

export async function GET(request: Request) {
  try {
    const { searchParams } = new URL(request.url)

    // Get query parameters with defaults
    const title = searchParams.get('title') || 'Documentation • Tempo'
    const description =
      searchParams.get('description') ||
      'Documentation for Tempo testnet and protocol specifications'
    const logoUrl = searchParams.get('logo') || ''
    const backgroundUrl = searchParams.get('background') || ''

    // Load fonts - matching explorer PR #279 setup
    try {
      const fonts = await loadFonts()

      return new ImageResponse(
        <OgImageTemplate
          title={title}
          description={description}
          backgroundUrl={backgroundUrl || undefined}
          {...(logoUrl ? { logoUrl } : {})}
        />,
        {
          width: 1200,
          height: 630,
          fonts: [
            { weight: 400, name: 'GeistMono', data: fonts.mono, style: 'normal' },
            { weight: 500, name: 'Inter', data: fonts.inter, style: 'normal' },
          ],
        },
      )
    } catch (fontError) {
      console.error('Font loading failed, using system fonts:', fontError)
      // Fallback to system fonts if custom fonts fail
      return new ImageResponse(
        <OgImageTemplate
          title={title}
          description={description}
          backgroundUrl={backgroundUrl || undefined}
          {...(logoUrl ? { logoUrl } : {})}
        />,
        {
          width: 1200,
          height: 630,
        },
      )
    }
  } catch (error) {
    console.error('OG image generation error:', error)
    // Return a simple error image
    return new ImageResponse(
      (
        <div
          style={{
            height: '100%',
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            backgroundColor: '#ffffff',
            fontSize: '32px',
            color: '#666666',
          }}
        >
          Documentation • Tempo
        </div>
      ),
      {
        width: 1200,
        height: 630,
      },
    )
  }
}
