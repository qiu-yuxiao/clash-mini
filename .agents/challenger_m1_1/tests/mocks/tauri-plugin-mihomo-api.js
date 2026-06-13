export class MihomoWebSocket {
  static connect_traffic() {
    return new MihomoWebSocket('traffic');
  }
  static connect_logs(logLevel) {
    return new MihomoWebSocket('logs-' + logLevel);
  }
  constructor(type) {
    this.type = type;
    this.closed = false;
    this.listeners = [];
  }
  close() {
    this.closed = true;
    return Promise.resolve();
  }
  addListener(cb) {
    this.listeners.push(cb);
  }
}
