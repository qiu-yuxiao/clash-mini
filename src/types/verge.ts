import { IVergeTestItem } from './profile'

export type ValidationOutcome =
  | { status: 'valid' | 'busy' }
  | { status: 'invalid'; kind: string; message: string }
  | { status: 'skipped'; reason: string }

export interface IVergeConfig {
  app_log_level?: 'trace' | 'debug' | 'info' | 'warn' | 'error' | string
  app_log_max_size?: number // KB
  app_log_max_count?: number
  language?: string
  env_type?: 'bash' | 'cmd' | 'powershell' | 'fish' | string
  startup_script?: string
  start_page?: string
  clash_core?: string
  theme_mode?: 'light' | 'dark' | 'system'
  traffic_graph?: boolean
  enable_memory_usage?: boolean
  enable_group_icon?: boolean
  pause_render_traffic_stats_on_blur?: boolean
  menu_icon?: 'monochrome' | 'colorful' | 'disable'
  menu_order?: string[]
  notice_position?: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'
  collapse_navbar?: boolean
  tray_icon?: 'monochrome' | 'colorful'
  enable_tray_speed?: boolean
  enable_always_on_top?: boolean

  enable_tun_mode?: boolean
  enable_auto_light_weight_mode?: boolean
  auto_light_weight_minutes?: number
  enable_auto_launch?: boolean
  enable_silent_start?: boolean
  enable_system_proxy?: boolean
  enable_dns_settings?: boolean
  proxy_auto_config?: boolean
  pac_file_content?: string
  proxy_host?: string
  enable_random_port?: boolean
  verge_mixed_port?: number
  verge_socks_port?: number
  verge_redir_port?: number
  verge_tproxy_port?: number
  verge_port?: number
  verge_redir_enabled?: boolean
  verge_tproxy_enabled?: boolean
  verge_socks_enabled?: boolean
  verge_http_enabled?: boolean
  enable_proxy_guard?: boolean
  enable_bypass_check?: boolean
  use_default_bypass?: boolean
  proxy_guard_duration?: number
  system_proxy_bypass?: string
  web_ui_list?: string[]
  theme_setting?: {
    primary_color?: string
    secondary_color?: string
    primary_text?: string
    secondary_text?: string
    info_color?: string
    error_color?: string
    warning_color?: string
    success_color?: string
    font_family?: string
    background_image?: string
    background_blend_mode?: string
    background_opacity?: number
  }
  auto_close_connection?: boolean
  auto_check_update?: boolean
  default_latency_test?: string
  enable_auto_delay_detection?: boolean
  auto_delay_detection_interval_minutes?: number
  enable_builtin_enhanced?: boolean
  auto_log_clean?: 0 | 1 | 2 | 3 | 4
  enable_auto_backup_schedule?: boolean
  auto_backup_interval_hours?: number
  auto_backup_on_change?: boolean
  proxy_layout_column?: number
  test_list?: IVergeTestItem[]
  webdav_url?: string
  webdav_username?: string
  webdav_password?: string
  home_cards?: Record<string, boolean>
  enable_hover_jump_navigator?: boolean
  hover_jump_navigator_delay?: number
  enable_external_controller?: boolean
  rule_fallback?: 'direct' | 'proxy' | 'addurl'
}

export interface IWebDavFile {
  filename: string
  href: string
  last_modified: string
  content_length: number
  content_type: string
  tag: string
}

export interface ILocalBackupFile {
  filename: string
  path: string
  last_modified: string
  content_length: number
}

export interface IWebDavConfig {
  url: string
  username: string
  password: string
}

// Traffic monitor types
