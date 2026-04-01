// @ts-ignore
import { prepare, layout } from '@chenglou/pretext';

export interface LayoutResult {
  height: number;
  lineCount: number;
}

const preparedCache = new Map<string, any>();

/**
 * Pre-calculate text layout data
 * @param id Unique ID for the text (e.g. article ID)
 * @param text The text to measure
 * @param font Font shorthand string (e.g. "16px Inter")
 */
export function prepareText(id: string, text: string, font: string) {
  const cacheKey = `${id}-${font}`;
  if (preparedCache.has(cacheKey)) {
    return preparedCache.get(cacheKey);
  }
  
  const prepared = prepare(text, font);
  preparedCache.set(cacheKey, prepared);
  return prepared;
}

/**
 * Calculate height for a prepared text at a given width
 * @param id Unique ID for the text
 * @param font Font shorthand string
 * @param width Container width
 * @param lineHeight Line height in pixels
 */
export function calculateHeight(id: string, font: string, width: number, lineHeight: number): LayoutResult {
  const cacheKey = `${id}-${font}`;
  const prepared = preparedCache.get(cacheKey);
  
  if (!prepared) {
    return { height: 0, lineCount: 0 };
  }
  
  return layout(prepared, width, lineHeight);
}

/**
 * Clear the layout cache
 */
export function clearLayoutCache() {
  preparedCache.clear();
}
