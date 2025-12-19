// Type declaration for Vite Plugin (vite is available at runtime via vocs)
type Plugin = {
  name: string
  enforce?: 'pre' | 'post'
  transform?: (code: string, id: string) => { code: string; map: string | null } | null
  [key: string]: unknown
}

import { 
  extractTitle, 
  extractDescription, 
  generateFrontmatter,
  truncateDescription,
  appendTempoBranding
} from './generate-seo-metadata'

/**
 * Vite plugin to auto-generate and update SEO metadata for MDX files
 * Always checks and updates metadata to ensure it stays in sync with content
 */
export function seoMetadataPlugin(): Plugin {
  return {
    name: 'vite-plugin-seo-metadata',
    enforce: 'pre',
    transform(code: string, id: string) {
      // Only process MDX files from pages directory
      if (!id.includes('/pages/') || !id.endsWith('.mdx')) {
        return null
      }

      // Check if frontmatter already exists
      const frontmatterMatch = code.match(/^---\n([\s\S]*?)\n---\n/)
      const hasTitle = frontmatterMatch?.[1]?.includes('title:') ?? false
      const hasDescription = frontmatterMatch?.[1]?.includes('description:') ?? false

      // Extract metadata - always extract to ensure consistency
      const fileName = id.split('/').pop() || id
      const title = extractTitle(code, fileName)
      
      // Get description - always process to ensure it ends with period and is properly formatted
      let description = ''
      if (frontmatterMatch && hasDescription && frontmatterMatch[1]) {
        const frontmatter = frontmatterMatch[1]
        // Match description that may span multiple lines or be on a single line
        const descMatch = frontmatter.match(/description:\s*(.+?)(?=\n\w+:|---|$)/s)
        if (descMatch && descMatch[1]) {
          let rawDesc = descMatch[1].trim()
          // Remove quotes if present
          if ((rawDesc.startsWith('"') && rawDesc.endsWith('"')) || 
              (rawDesc.startsWith("'") && rawDesc.endsWith("'"))) {
            rawDesc = rawDesc.slice(1, -1)
          }
          description = rawDesc.trim()
        }
      }
      
      // If no description in frontmatter, extract from content
      if (!description) {
        description = extractDescription(code)
      } else {
        // Check if existing description is malformed (ends mid-word, no period, etc.)
        const endsWithPeriod = description.endsWith('.')
        const endsWithSpace = description.endsWith(' ')
        const lastChar = description.trim().slice(-1)
        const isAlphanumeric = /[a-zA-Z0-9]/.test(lastChar)
        const isMalformed = !endsWithPeriod && isAlphanumeric && !endsWithSpace
        
        // Check if existing description is incomplete (has text after first sentence)
        const descFirstPeriod = description.indexOf('.')
        const textAfterFirstPeriod = descFirstPeriod > 10 ? 
          description.substring(descFirstPeriod + 1).trim() : ''
        
        // If description is malformed or has significant text after the first period,
        // extract a fresh one from content
        if (isMalformed || textAfterFirstPeriod.length > 3) {
          description = extractDescription(code)
        } else {
          // Always apply truncation to ensure it ends with period and is within limit
          description = truncateDescription(description)
        }
      }

      // Check if we actually need to update (only skip if both exist AND are already correct)
      let needsUpdate = true
      if (hasTitle && hasDescription && frontmatterMatch?.[1]) {
        // Check if title and description match what we would generate
        const existingTitleMatch = frontmatterMatch[1].match(/title:\s*(.+?)(?=\n\w+:|$)/s)
        let existingTitle = existingTitleMatch?.[1]?.trim() || ''
        // Remove quotes from existing title for comparison
        if ((existingTitle.startsWith('"') && existingTitle.endsWith('"')) || 
            (existingTitle.startsWith("'") && existingTitle.endsWith("'"))) {
          existingTitle = existingTitle.slice(1, -1).trim()
        }
        
        // Check if existing title has " • Tempo" branding
        const tempoSuffix = ' • Tempo'
        const hasBranding = existingTitle.endsWith(tempoSuffix) || existingTitle.endsWith(' • Tempo') || existingTitle.endsWith(' · Tempo')
        
        // Normalize existing title to ensure it has " • Tempo" branding for comparison
        const normalizedExistingTitle = appendTempoBranding(existingTitle)
        
        const existingDescMatch = frontmatterMatch[1].match(/description:\s*(.+?)(?=\n\w+:|---|$)/s)
        let existingDesc = existingDescMatch?.[1]?.trim() || ''
        // Remove quotes from existing description for comparison
        if ((existingDesc.startsWith('"') && existingDesc.endsWith('"')) || 
            (existingDesc.startsWith("'") && existingDesc.endsWith("'"))) {
          existingDesc = existingDesc.slice(1, -1).trim()
        }
        
        // Check if existing description is malformed
        const existingEndsWithPeriod = existingDesc.endsWith('.')
        const existingEndsWithSpace = existingDesc.endsWith(' ')
        const existingLastChar = existingDesc.trim().slice(-1)
        const existingIsAlphanumeric = /[a-zA-Z0-9]/.test(existingLastChar)
        const existingIsMalformed = !existingEndsWithPeriod && existingIsAlphanumeric && !existingEndsWithSpace
        
        // Check if existing description is properly truncated (ends at first sentence)
        const normalizedExistingDesc = truncateDescription(existingDesc)
        const wouldChangeAfterTruncation = normalizedExistingDesc !== existingDesc
        
        // Check if there's significant text after the first period
        const existingDescFirstPeriod = existingDesc.indexOf('.')
        const existingTextAfterFirstPeriod = existingDescFirstPeriod > 10 ? 
          existingDesc.substring(existingDescFirstPeriod + 1).trim() : ''
        
        // If truncateDescription would change it, or there's text after the first period, or it's malformed, it's not properly truncated
        const isProperlyTruncated = !existingIsMalformed &&
                                    !wouldChangeAfterTruncation && 
                                    existingDescFirstPeriod > 10 && 
                                    (existingTextAfterFirstPeriod === '' || existingTextAfterFirstPeriod.length <= 3)
        
        // Check if the actual existing description matches what we would generate
        const actualDescMatches = existingDesc === description
        
        // Only skip if ALL of these are true:
        // 1. Title matches (after normalization) AND
        // 2. Title already has branding AND
        // 3. Actual description in file matches what we would generate (exact match) AND
        // 4. Description ends with period AND
        // 5. Description is properly truncated at a sentence boundary
        if (normalizedExistingTitle === title && hasBranding && 
            actualDescMatches && description.endsWith('.') &&
            isProperlyTruncated) {
          needsUpdate = false
        }
      }

      // Only update if needed
      if (!needsUpdate) {
        return null
      }

      // Generate new content
      let newCode = code

      if (frontmatterMatch) {
        // Update existing frontmatter
        const existingFrontmatter = frontmatterMatch[1]
        const newFrontmatter = generateFrontmatter(title, description, existingFrontmatter)
        newCode = code.replace(/^---\n[\s\S]*?\n---\n/, newFrontmatter)
      } else {
        // Add new frontmatter
        const newFrontmatter = generateFrontmatter(title, description)
        newCode = newFrontmatter + code
      }

      return {
        code: newCode,
        map: null,
      }
    },
  }
}

