use std::net::SocketAddr;
use std::time::Instant;

use jsonrpsee::server::logger::{self, HttpRequest, MethodKind, Params, TransportProtocol};

#[derive(Clone)]
pub(crate) struct Logger;

// 实现logger::Logger 回调函数以定制日志记录行为
impl logger::Logger for Logger {
    // 指定Instant类型用于时间测量
    type Instant = Instant;

    /// 当建立连接时调用
    ///
    /// # Parameters
    ///
    /// * `remote_addr`: 远程地址信息
    /// * `request`: HTTP请求对象
    /// * `_t`: 传输协议类型（未使用）
    fn on_connect(&self, remote_addr: SocketAddr, request: &HttpRequest, _t: TransportProtocol) {
        // 记录连接建立时的日志，包含远程地址和请求头信息
        tracing::info!(
            "[Logger::on_connect] remote_addr {:?}, headers: {:?}",
            remote_addr,
            request
        );
    }

    /// 当请求开始时调用，用于记录请求开始时间
    ///
    /// # Parameters
    ///
    /// * `_t`: 传输协议类型（未使用）
    ///
    /// # Returns
    ///
    /// * `Self::Instant`: 请求开始的瞬间时间
    fn on_request(&self, _t: TransportProtocol) -> Self::Instant {
        // 记录请求开始时的日志
        tracing::info!("[Logger::on_request]");
        // 返回当前时间作为请求开始时间
        Instant::now()
    }

    /// 当方法被调用时调用
    ///
    /// # Parameters
    ///
    /// * `name`: 方法名称
    /// * `params`: 方法参数
    /// * `kind`: 方法类型
    /// * `_t`: 传输协议类型（未使用）
    fn on_call(&self, name: &str, params: Params, kind: MethodKind, _t: TransportProtocol) {
        // 记录方法调用日志，包括方法名、参数和类型
        tracing::info!(
            "[Logger::on_call] method: '{}', params: {:?}, kind: {}",
            name,
            params,
            kind
        );
    }

    /// 当方法执行结果出来时调用
    ///
    /// # Parameters
    ///
    /// * `name`: 方法名称
    /// * `success`: 方法执行是否成功
    /// * `started_at`: 方法开始执行的时间
    /// * `_t`: 传输协议类型（未使用）
    fn on_result(
        &self,
        name: &str,
        success: bool,
        started_at: Self::Instant,
        _t: TransportProtocol,
    ) {
        // 记录方法执行结果日志，包括方法名、执行是否成功和耗时
        tracing::info!(
            "[Logger::on_result] '{}', worked? {}, time elapsed {:?}",
            name,
            success,
            started_at.elapsed()
        );
    }

    /// 当响应生成时调用
    ///
    /// # Parameters
    ///
    /// * `result`: 响应结果字符串
    /// * `started_at`: 响应开始的时间
    /// * `_t`: 传输协议类型（未使用）
    fn on_response(&self, result: &str, started_at: Self::Instant, _t: TransportProtocol) {
        // 记录响应生成日志，包括响应结果和耗时
        tracing::info!(
            "[Logger::on_response] result: {}, time elapsed {:?}",
            result,
            started_at.elapsed()
        );
    }

    /// 当断开连接时调用
    ///
    /// # Parameters
    ///
    /// * `remote_addr`: 远程地址信息
    /// * `_t`: 传输协议类型（未使用）
    fn on_disconnect(&self, remote_addr: SocketAddr, _t: TransportProtocol) {
        // 记录断开连接日志，包含远程地址信息
        tracing::info!("[Logger::on_disconnect] remote_addr: {:?}", remote_addr);
    }
}
