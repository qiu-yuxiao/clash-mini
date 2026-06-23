/**
 * 判断一个节点是否是广告、说明或假节点
 */
export const isDummyNode = (name: string): boolean => {
  if (!name) return true
  const lower = name.toLowerCase()
  return (
    lower.includes('流量') ||
    lower.includes('过期时间') ||
    lower.includes('网址') ||
    lower.includes('官网') ||
    lower.includes('剩余') ||
    lower.includes('expire') ||
    lower.includes('traffic') ||
    lower.includes('website') ||
    lower.includes('套餐到期') ||
    lower.includes('续费') ||
    lower.includes('公告') ||
    lower.includes('购买') ||
    lower.includes('subscribe') ||
    lower.includes('群') ||
    lower.startsWith('dummy') ||
    lower.startsWith('(dummy)') ||
    lower.includes('dummy') ||
    name === 'DIRECT' ||
    name === 'REJECT' ||
    name === 'COMPATIBLE' ||
    // 升级匹配项，要求带上协议分隔符，防止误伤以 "http-hk" 等命名的真实代理协议节点
    lower.includes('http://') ||
    lower.includes('https://')
  )
}
