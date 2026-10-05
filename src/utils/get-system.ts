// get the system os
// Clash Mini 仅支持 Windows 10 x64，故恒返回 'windows'。
// 保留函数以兼容既有调用点（CSS 类名、条件渲染等）。
export default function getSystem() {
  return 'windows' as const
}
