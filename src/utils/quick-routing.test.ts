import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  readFile: vi.fn(),
  writeFile: vi.fn(),
}))

vi.mock('@/services/cmds', () => ({
  readProfileFile: vi.fn(),
  saveProfileFile: vi.fn(),
  enhanceProfiles: vi.fn(),
  beginLongOperation: vi.fn(),
  endLongOperation: vi.fn(),
}))

vi.mock('@/services/notice-service', () => ({
  showNotice: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}))

import {
  enhanceProfiles,
  readProfileFile,
  saveProfileFile,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import {
  addQuickRoutingRule,
  addQuickRoutingRules,
  buildExactQuickRoutingRule,
  buildProxyRule,
  buildQuickRuleFromInput,
  extractRegistrableDomain,
  normalizeQuickTargetHost,
  parseManagedMergeRuleLine,
  parseManagedMergeRulesFromYaml,
  removePrependRules,
  removeQuickRoutingRule,
  upsertPrependRule,
} from './quick-routing'

describe('quick-routing 精确规则生成（BUG-367）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('封锁或分流 play.googleapis.com 必须严格生成精确 DOMAIN，禁止扩域或截断父域', () => {
    expect(
      buildExactQuickRoutingRule('domain', 'play.googleapis.com', 'REJECT'),
    ).toBe('DOMAIN,play.googleapis.com,REJECT')

    expect(
      buildExactQuickRoutingRule('domain', 'a.b.play.googleapis.com', 'REJECT'),
    ).toBe('DOMAIN,a.b.play.googleapis.com,REJECT')

    expect(
      buildExactQuickRoutingRule('domain', 'PLAY.GOOGLEAPIS.COM.', 'PROXY'),
    ).toBe('DOMAIN,play.googleapis.com,PROXY')

    expect(
      buildExactQuickRoutingRule('domain', 'example.co.uk:443', 'DIRECT'),
    ).toBe('DOMAIN,example.co.uk,DIRECT')
  })

  it('IPv4 与 IPv6 应分别精确生成 /32 IP-CIDR 与 /128 IP-CIDR6', () => {
    expect(
      buildExactQuickRoutingRule('domain', '142.250.190.46', 'PROXY'),
    ).toBe('IP-CIDR,142.250.190.46/32,PROXY')

    expect(
      buildExactQuickRoutingRule('domain', '2001:4860:4860::8888', 'DIRECT'),
    ).toBe('IP-CIDR6,2001:4860:4860::8888/128,DIRECT')

    expect(
      buildExactQuickRoutingRule(
        'domain',
        '[2606:4700:4700::1111]:443',
        'REJECT',
      ),
    ).toBe('IP-CIDR6,2606:4700:4700::1111/128,REJECT')

    expect(buildExactQuickRoutingRule('domain', 'fe80::1%eth0', 'DIRECT')).toBe(
      'IP-CIDR6,fe80::1/128,DIRECT',
    )
  })

  it('进程规则应精确保留进程名', () => {
    expect(
      buildExactQuickRoutingRule('process', ' GoogleUpdate.exe ', 'REJECT'),
    ).toBe('PROCESS-NAME,GoogleUpdate.exe,REJECT')
  })

  it('extractRegistrableDomain 精准识别标准根域、多级 ccTLD 与共享后缀，并拦截裸公共后缀与 IP', () => {
    expect(extractRegistrableDomain('console.ap-tokyo-1.oraclecloud.com')).toBe(
      'oraclecloud.com',
    )
    expect(extractRegistrableDomain('oracle.com')).toBe('oracle.com')
    expect(extractRegistrableDomain('news.bbc.co.uk')).toBe('bbc.co.uk')
    expect(extractRegistrableDomain('login.example.com.cn')).toBe(
      'example.com.cn',
    )
    expect(extractRegistrableDomain('sub.foo.co.jp')).toBe('foo.co.jp')
    expect(extractRegistrableDomain('docs.myuser.github.io')).toBe(
      'myuser.github.io',
    )
    expect(extractRegistrableDomain('api.myapp.workers.dev')).toBe(
      'myapp.workers.dev',
    )
    expect(extractRegistrableDomain('co.uk')).toBeNull()
    expect(extractRegistrableDomain('com.cn')).toBeNull()
    expect(extractRegistrableDomain('workers.dev')).toBeNull()
    expect(extractRegistrableDomain('localhost')).toBeNull()
    expect(extractRegistrableDomain('1.2.3.4')).toBeNull()
    expect(extractRegistrableDomain('2001:db8::1')).toBeNull()
  })

  it('添加规则 buildQuickRuleFromInput / buildProxyRule：支持 PROXY/DIRECT/REJECT，裸根域名与通配符生成 DOMAIN-SUFFIX，具体子域名严格生成 DOMAIN', () => {
    expect(buildProxyRule('https://play.googleapis.com/log?x=1')).toBe(
      'DOMAIN,play.googleapis.com,PROXY',
    )
    expect(buildProxyRule('www.google.com')).toBe('DOMAIN,www.google.com,PROXY')
    expect(buildProxyRule('*.googleapis.com')).toBe(
      'DOMAIN-SUFFIX,googleapis.com,PROXY',
    )
    expect(buildProxyRule('.example.org')).toBe(
      'DOMAIN-SUFFIX,example.org,PROXY',
    )
    expect(buildProxyRule('1.2.3.4')).toBe('IP-CIDR,1.2.3.4/32,PROXY')
    expect(buildProxyRule('192.168.10.0/24')).toBe(
      'IP-CIDR,192.168.10.0/24,PROXY',
    )
    expect(buildProxyRule('2001:db8::1')).toBe('IP-CIDR6,2001:db8::1/128,PROXY')
    expect(buildProxyRule('2001:db8::/32')).toBe('IP-CIDR6,2001:db8::/32,PROXY')
    expect(buildProxyRule('localhost')).toBeNull()

    expect(buildQuickRuleFromInput('oracle.com', 'DIRECT')).toBe(
      'DOMAIN-SUFFIX,oracle.com,DIRECT',
    )
    expect(buildQuickRuleFromInput('https://oraclecloud.com/', 'DIRECT')).toBe(
      'DOMAIN-SUFFIX,oraclecloud.com,DIRECT',
    )
    expect(buildQuickRuleFromInput('bbc.co.uk', 'DIRECT')).toBe(
      'DOMAIN-SUFFIX,bbc.co.uk,DIRECT',
    )
    expect(buildQuickRuleFromInput('*.oraclecloud.com', 'DIRECT')).toBe(
      'DOMAIN-SUFFIX,oraclecloud.com,DIRECT',
    )
    expect(buildQuickRuleFromInput('*.co.uk', 'DIRECT')).toBeNull()
    expect(buildQuickRuleFromInput('play.googleapis.com', 'REJECT')).toBe(
      'DOMAIN,play.googleapis.com,REJECT',
    )
    expect(buildQuickRuleFromInput('1.2.3.4', 'REJECT')).toBe(
      'IP-CIDR,1.2.3.4/32,REJECT',
    )
  })

  it('normalizeQuickTargetHost 保留 IPv6 冒号且仅剥离单冒号端口', () => {
    expect(normalizeQuickTargetHost('[2001:db8::1]:8443')).toBe('2001:db8::1')
    expect(normalizeQuickTargetHost('1.1.1.1:443')).toBe('1.1.1.1')
    expect(normalizeQuickTargetHost('2001:4860::8888')).toBe('2001:4860::8888')
  })

  it('解析、去重覆写与移除 Merge prepend-rules', () => {
    const parsed = parseManagedMergeRuleLine(
      '  ip-cidr6, 2001:DB8::1/128, reject, no-resolve ',
    )
    expect(parsed).toEqual({
      raw: 'ip-cidr6, 2001:DB8::1/128, reject, no-resolve',
      ruleType: 'IP-CIDR6',
      value: '2001:db8::1/128',
      matchKey: '2001:db8::1',
      target: 'REJECT',
      extra: 'no-resolve',
    })

    const yamlText = [
      'prepend-rules:',
      '  - DOMAIN,play.googleapis.com,REJECT',
      '  - DOMAIN-SUFFIX,googleapis.com,PROXY',
      '  - IP-CIDR,1.2.3.4/32,DIRECT,no-resolve',
      '  - PROCESS-NAME,chrome.exe,PROXY',
      '  - MATCH,DIRECT',
    ].join('\n')

    const parsedList = parseManagedMergeRulesFromYaml(yamlText)
    expect(parsedList).toHaveLength(4)
    expect(parsedList.map((r) => r.ruleType)).toEqual([
      'DOMAIN',
      'DOMAIN-SUFFIX',
      'IP-CIDR',
      'PROCESS-NAME',
    ])

    const updated = upsertPrependRule(
      ['DOMAIN,play.googleapis.com,REJECT', 'IP-CIDR,1.2.3.4/32,DIRECT'],
      'DOMAIN,play.googleapis.com,PROXY',
    )
    expect(updated).toEqual([
      'DOMAIN,play.googleapis.com,PROXY',
      'IP-CIDR,1.2.3.4/32,DIRECT',
    ])

    const removed = removePrependRules(updated, {
      rawRule: 'DOMAIN,play.googleapis.com,PROXY',
    })
    expect(removed.removed).toEqual(['DOMAIN,play.googleapis.com,PROXY'])
    expect(removed.nextRules).toEqual(['IP-CIDR,1.2.3.4/32,DIRECT'])
  })

  it('addQuickRoutingRule 写入精确规则并避免覆盖兄弟子域；removeQuickRoutingRule 可精确撤销', async () => {
    vi.mocked(readProfileFile).mockResolvedValue(
      [
        'prepend-rules:',
        '  - DOMAIN,daily-cloudcode-pa.googleapis.com,PROXY',
        '',
      ].join('\n'),
    )
    vi.mocked(saveProfileFile).mockResolvedValue(undefined as never)
    vi.mocked(enhanceProfiles).mockResolvedValue(undefined as never)

    const addedRule = await addQuickRoutingRule(
      'domain',
      'play.googleapis.com',
      'REJECT',
    )
    expect(addedRule).toBe('DOMAIN,play.googleapis.com,REJECT')
    expect(saveProfileFile).toHaveBeenCalledTimes(1)
    const savedYaml = vi.mocked(saveProfileFile).mock.calls[0][1]
    expect(savedYaml).toContain('DOMAIN,play.googleapis.com,REJECT')
    expect(savedYaml).toContain(
      'DOMAIN,daily-cloudcode-pa.googleapis.com,PROXY',
    )
    expect(savedYaml).not.toContain('DOMAIN-SUFFIX,googleapis.com')
    expect(enhanceProfiles).toHaveBeenCalledTimes(1)
    expect(showNotice.success).toHaveBeenCalledWith(
      'profiles.page.feedback.notifications.quickRuleAdded',
      { rule: 'DOMAIN,play.googleapis.com,REJECT' },
      2500,
    )

    vi.mocked(readProfileFile).mockResolvedValue(savedYaml)
    const removed = await removeQuickRoutingRule({
      rawRule: 'DOMAIN,play.googleapis.com,REJECT',
    })
    expect(removed).toBe('DOMAIN,play.googleapis.com,REJECT')
    expect(saveProfileFile).toHaveBeenCalledTimes(2)
    const afterRemoveYaml = vi.mocked(saveProfileFile).mock.calls[1][1]
    expect(afterRemoveYaml).not.toContain('DOMAIN,play.googleapis.com,REJECT')
    expect(afterRemoveYaml).toContain(
      'DOMAIN,daily-cloudcode-pa.googleapis.com,PROXY',
    )
    expect(showNotice.success).toHaveBeenCalledWith(
      'profiles.page.feedback.notifications.quickRuleRemoved',
      { rule: 'DOMAIN,play.googleapis.com,REJECT' },
      2500,
    )
  })

  it('addQuickRoutingRules 批量写入精确 DOMAIN 与显式通配符，并去重相同规则', async () => {
    vi.mocked(readProfileFile).mockResolvedValue(
      'prepend-rules:\n  - DOMAIN,old.com,DIRECT\n',
    )
    vi.mocked(saveProfileFile).mockResolvedValue(undefined as never)
    vi.mocked(enhanceProfiles).mockResolvedValue(undefined as never)

    const count = await addQuickRoutingRules(
      'https://play.googleapis.com/a\n*.cdn.example.com\n2001:db8::1\nhttps://play.googleapis.com/b',
    )
    expect(count).toBe(3)
    const savedYaml = vi.mocked(saveProfileFile).mock.calls[0][1]
    expect(savedYaml).toContain('DOMAIN,play.googleapis.com,PROXY')
    expect(savedYaml).toContain('DOMAIN-SUFFIX,cdn.example.com,PROXY')
    expect(savedYaml).toContain('IP-CIDR6,2001:db8::1/128,PROXY')
  })

  it('addQuickRoutingRules 支持 DIRECT 与 REJECT 目标（裸根域名自动整站生效）', async () => {
    vi.mocked(readProfileFile).mockResolvedValue('prepend-rules: []\n')
    vi.mocked(saveProfileFile).mockResolvedValue(true as never)
    vi.mocked(enhanceProfiles).mockResolvedValue(true as never)

    const directCount = await addQuickRoutingRules(
      'oracle.com\noraclecloud.com\n10.0.0.5',
      'DIRECT',
    )
    expect(directCount).toBe(3)
    const directYaml = vi.mocked(saveProfileFile).mock.calls[0][1]
    expect(directYaml).toContain('DOMAIN-SUFFIX,oracle.com,DIRECT')
    expect(directYaml).toContain('DOMAIN-SUFFIX,oraclecloud.com,DIRECT')
    expect(directYaml).toContain('IP-CIDR,10.0.0.5/32,DIRECT')

    const rejectCount = await addQuickRoutingRules(
      'play.googleapis.com\n*.ads.example.com',
      'REJECT',
    )
    expect(rejectCount).toBe(2)
    const rejectYaml = vi.mocked(saveProfileFile).mock.calls[1][1]
    expect(rejectYaml).toContain('DOMAIN,play.googleapis.com,REJECT')
    expect(rejectYaml).toContain('DOMAIN-SUFFIX,ads.example.com,REJECT')
  })

  it('addQuickRoutingRule 写入 DOMAIN-SUFFIX 自动收编被覆盖子域名规则，且允许后续精准子域名置顶覆写例外', async () => {
    vi.mocked(readProfileFile).mockResolvedValue(
      [
        'prepend-rules:',
        '  - DOMAIN,login.oraclecloud.com,DIRECT',
        '  - DOMAIN,console.ap-tokyo-1.oraclecloud.com,PROXY',
        '  - DOMAIN,keep.other.com,DIRECT',
        '',
      ].join('\n'),
    )
    vi.mocked(saveProfileFile).mockResolvedValue(true as never)
    vi.mocked(enhanceProfiles).mockResolvedValue(true as never)

    const suffixRule = await addQuickRoutingRule(
      'domain-suffix',
      'oraclecloud.com',
      'DIRECT',
    )
    expect(suffixRule).toBe('DOMAIN-SUFFIX,oraclecloud.com,DIRECT')
    const afterSuffixYaml = vi.mocked(saveProfileFile).mock.calls[0][1]
    expect(afterSuffixYaml).toContain('DOMAIN-SUFFIX,oraclecloud.com,DIRECT')
    expect(afterSuffixYaml).not.toContain('login.oraclecloud.com')
    expect(afterSuffixYaml).not.toContain('console.ap-tokyo-1.oraclecloud.com')
    expect(afterSuffixYaml).toContain('DOMAIN,keep.other.com,DIRECT')

    vi.mocked(readProfileFile).mockResolvedValue(afterSuffixYaml)
    const exceptionRule = await addQuickRoutingRule(
      'domain',
      'telemetry.oraclecloud.com',
      'REJECT',
    )
    expect(exceptionRule).toBe('DOMAIN,telemetry.oraclecloud.com,REJECT')
    const afterExceptionYaml = vi.mocked(saveProfileFile).mock.calls[1][1]
    const parsedException = parseManagedMergeRulesFromYaml(afterExceptionYaml)
    expect(parsedException.map((r) => r.raw)).toEqual([
      'DOMAIN,telemetry.oraclecloud.com,REJECT',
      'DOMAIN-SUFFIX,oraclecloud.com,DIRECT',
      'DOMAIN,keep.other.com,DIRECT',
    ])
  })
})

describe('custom routing backup & restore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('serializeCustomRoutingBackup 与 parseCustomRoutingBackup 支持往返解析、去重并过滤无效行', async () => {
    const { serializeCustomRoutingBackup, parseCustomRoutingBackup } =
      await import('./quick-routing')

    const yamlText = serializeCustomRoutingBackup([
      'DOMAIN,alpha.example.com,DIRECT',
      'DOMAIN-SUFFIX,beta.example.com,REJECT',
      'DOMAIN,alpha.example.com,DIRECT',
    ])

    expect(yamlText).toContain('prepend-rules:')
    expect(parseCustomRoutingBackup(yamlText)).toEqual([
      'DOMAIN,alpha.example.com,DIRECT',
      'DOMAIN-SUFFIX,beta.example.com,REJECT',
    ])

    expect(
      parseCustomRoutingBackup(`
rules:
  - "# comment line"
  - "  "
  - "INVALID_RULE"
  - "IP-CIDR,1.2.3.0/24,DIRECT,no-resolve"
  - "IP-CIDR,1.2.3.0/24,PROXY,no-resolve"
`),
    ).toEqual(['IP-CIDR,1.2.3.0/24,DIRECT,no-resolve'])

    expect(parseCustomRoutingBackup('not: [valid: yaml')).toEqual([])
    expect(parseCustomRoutingBackup('')).toEqual([])
  })

  it('exportCustomRoutingRules 在无规则时提示空状态，有规则时写入用户指定路径', async () => {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const { writeFile } = await import('@tauri-apps/plugin-fs')
    const { readProfileFile } = await import('@/services/cmds')
    const { showNotice } = await import('@/services/notice-service')
    const { exportCustomRoutingRules } = await import('./quick-routing')

    vi.mocked(readProfileFile).mockResolvedValueOnce('prepend-rules: []\n')
    const emptyResult = await exportCustomRoutingRules()
    expect(emptyResult).toBe(0)
    expect(showNotice.info).toHaveBeenCalledWith(
      'settings.mini.routesExportEmpty',
    )
    expect(save).not.toHaveBeenCalled()
    expect(writeFile).not.toHaveBeenCalled()

    vi.mocked(readProfileFile).mockResolvedValueOnce(
      'prepend-rules:\n  - DOMAIN,foo.com,DIRECT\n  - DOMAIN-SUFFIX,bar.com,REJECT\n',
    )
    vi.mocked(save).mockResolvedValueOnce(null)
    const cancelledResult = await exportCustomRoutingRules()
    expect(cancelledResult).toBeNull()
    expect(writeFile).not.toHaveBeenCalled()

    vi.mocked(readProfileFile).mockResolvedValueOnce(
      'prepend-rules:\n  - DOMAIN,foo.com,DIRECT\n  - DOMAIN-SUFFIX,bar.com,REJECT\n',
    )
    vi.mocked(save).mockResolvedValueOnce('D:\\backup\\routes.yaml')
    vi.mocked(writeFile).mockResolvedValueOnce(undefined)

    const exportedCount = await exportCustomRoutingRules()
    expect(exportedCount).toBe(2)
    expect(writeFile).toHaveBeenCalledTimes(1)
    const [writtenPath, writtenBytes] = vi.mocked(writeFile).mock.calls[0]
    expect(writtenPath).toBe('D:\\backup\\routes.yaml')
    const decoded = new TextDecoder('utf-8').decode(writtenBytes as Uint8Array)
    expect(decoded).toContain('DOMAIN,foo.com,DIRECT')
    expect(decoded).toContain('DOMAIN-SUFFIX,bar.com,REJECT')
    expect(showNotice.success).toHaveBeenCalledWith(
      'settings.mini.routesExportSuccess',
      { count: 2 },
    )
  })

  it('importCustomRoutingRules 支持取消、空文件拦截与合并写回 Merge 并重载配置', async () => {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const { readFile } = await import('@tauri-apps/plugin-fs')
    const {
      readProfileFile,
      saveProfileFile,
      enhanceProfiles,
      beginLongOperation,
      endLongOperation,
    } = await import('@/services/cmds')
    const { showNotice } = await import('@/services/notice-service')
    const { importCustomRoutingRules } = await import('./quick-routing')

    vi.mocked(open).mockResolvedValueOnce(null)
    expect(await importCustomRoutingRules()).toBeNull()
    expect(readFile).not.toHaveBeenCalled()

    vi.mocked(open).mockResolvedValueOnce('D:\\backup\\empty.yaml')
    vi.mocked(readFile).mockResolvedValueOnce(
      new TextEncoder().encode('version: 1\nprepend-rules: []\n'),
    )
    expect(await importCustomRoutingRules()).toBe(0)
    expect(showNotice.info).toHaveBeenCalledWith(
      'settings.mini.routesImportEmpty',
    )
    expect(saveProfileFile).not.toHaveBeenCalled()

    vi.mocked(open).mockResolvedValueOnce('D:\\backup\\routes.yaml')
    vi.mocked(readFile).mockResolvedValueOnce(
      new TextEncoder().encode(
        'version: 1\nprepend-rules:\n  - DOMAIN,first.com,DIRECT\n  - DOMAIN,second.com,REJECT\n',
      ),
    )
    vi.mocked(readProfileFile).mockResolvedValueOnce(
      'prepend-rules:\n  - DOMAIN,first.com,Proxy\n  - DOMAIN,keep.com,DIRECT\n',
    )
    vi.mocked(saveProfileFile).mockResolvedValueOnce(true)

    const importedCount = await importCustomRoutingRules()
    expect(importedCount).toBe(2)
    expect(saveProfileFile).toHaveBeenCalledTimes(1)
    const [profileId, savedYaml] = vi.mocked(saveProfileFile).mock.calls[0]
    expect(profileId).toBe('Merge')
    expect(savedYaml).toContain('DOMAIN,first.com,DIRECT')
    expect(savedYaml).toContain('DOMAIN,second.com,REJECT')
    expect(savedYaml).toContain('DOMAIN,keep.com,DIRECT')
    expect(savedYaml).not.toContain('DOMAIN,first.com,Proxy')
    expect(beginLongOperation).toHaveBeenCalledTimes(1)
    // saveProfileFile('Merge', ...) 已直通触发运行时重载时，禁止再重复调一次 enhanceProfiles()（BUG-409）
    expect(enhanceProfiles).not.toHaveBeenCalled()
    expect(endLongOperation).toHaveBeenCalledTimes(1)
    expect(showNotice.success).toHaveBeenCalledWith(
      'settings.mini.routesImportSuccess',
      { count: 2 },
    )

    // 若 saveProfileFile 未直通生效（如返回 false），仍须兜底触发 enhanceProfiles
    vi.mocked(readProfileFile).mockResolvedValueOnce('prepend-rules: []\n')
    vi.mocked(saveProfileFile).mockResolvedValueOnce(false)
    vi.mocked(enhanceProfiles).mockResolvedValueOnce(true)
    vi.mocked(readProfileFile) // keep queue clear
    vi.mocked(open).mockResolvedValueOnce('D:\\backup\\routes.yaml')
    vi.mocked(readFile).mockResolvedValueOnce(
      new TextEncoder().encode(
        'version: 1\nprepend-rules:\n  - DOMAIN,third.com,DIRECT\n',
      ),
    )
    expect(await importCustomRoutingRules()).toBe(1)
    expect(enhanceProfiles).toHaveBeenCalledTimes(1)
  })
})
