use reqwest::blocking::{Client, Response};
use std::io::{self, Read};

/// 読み取り操作と連動して進捗を報告するラッパー
pub struct ProgressReader<R: Read, F: FnMut(u64, u64) + Send + 'static> {
    inner: R,
    current: u64,
    total: u64,
    on_progress: F,
}

impl<R: Read, F: FnMut(u64, u64) + Send + 'static> ProgressReader<R, F> {
    pub fn new(inner: R, total: u64, on_progress: F) -> Self {
        Self {
            inner,
            current: 0,
            total,
            on_progress,
        }
    }
}

impl<R: Read, F: FnMut(u64, u64) + Send + 'static> Read for ProgressReader<R, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.current += n as u64;
            (self.on_progress)(self.current, self.total);
        }
        Ok(n)
    }
}

/// 物理的なデータ供給（HTTPリクエスト等）を抽象化する内部トレイト
pub trait InstallProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// 指定されたURLからデータをストリームとして取得する。
    fn fetch(&self, url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error>;
}

/// reqwest を使用した本物の HTTP プロバイダ
pub struct HttpProvider {
    client: Client,
}

impl HttpProvider {
    /// HttpProvider を作成。Client 構築失敗時はエラーを返す（panic しない）。
    pub fn try_new() -> Result<Self, reqwest::Error> {
        let client = Client::builder().user_agent("typstlab-installer").build()?;
        Ok(Self { client })
    }
}

impl InstallProvider for HttpProvider {
    type Error = reqwest::Error;

    fn fetch(&self, url: &str) -> Result<(Box<dyn Read + Send>, u64), Self::Error> {
        let resp: Response = self.client.get(url).send()?;
        let size = resp.content_length().unwrap_or(0);
        Ok((Box::new(resp), size))
    }
}

pub mod docs;
pub mod typst;

pub use docs::{DocsInstallError, DocsInstaller};
pub use typst::{TypstInstallError, TypstInstaller};
