/** 设置页共用的小工具：滑块进度百分比与字节格式化。 */

export function pct(v: number, min: number, max: number): string {
  return String(((v - min) / (max - min)) * 100);
}

export function fmtSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / 1024 / 1024).toFixed(2) + ' MB';
}
