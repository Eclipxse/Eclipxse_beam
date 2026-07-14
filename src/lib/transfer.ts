export interface ChunkRange {
  start: number;
  end: number;
}

export const DEFAULT_CHUNK_SIZE = 64 * 1024;

export function getChunkRanges(
  fileSize: number,
  chunkSize = DEFAULT_CHUNK_SIZE,
): ChunkRange[] {
  if (!Number.isFinite(fileSize) || fileSize <= 0) return [];
  if (!Number.isFinite(chunkSize) || chunkSize <= 0) {
    throw new Error('Chunk size must be a positive number.');
  }

  const ranges: ChunkRange[] = [];
  for (let start = 0; start < fileSize; start += chunkSize) {
    ranges.push({ start, end: Math.min(start + chunkSize, fileSize) });
  }
  return ranges;
}
