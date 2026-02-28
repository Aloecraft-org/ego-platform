
--- a/src/fs.rs
+++ b/src/fs.rs
``` rs
  1:// aloeplatform/src/fs.rs
  2:
  3:use std::path::Path;
  4:use std::io;
  5:
  6:// === Browser-specific implementation ===
  7:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
  8:mod browser_fs {
  9:    use std::path::Path;
 10:    use std::io::{self, ErrorKind};
 11:    use web_sys::window;
 12:    
 13:    const STORAGE_PREFIX: &str = "ego2_fs_";
 14:    
 15:    fn get_storage() -> io::Result<web_sys::Storage> {
 16:        window()
 17:            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No window object"))?
 18:            .local_storage()
 19:            .map_err(|_| io::Error::new(ErrorKind::Other, "localStorage not available"))?
 20:            .ok_or_else(|| io::Error::new(ErrorKind::Other, "localStorage is null"))
 21:    }
 22:    
 23:    fn path_to_key(path: &Path) -> String {
 24:        format!("{}{}", STORAGE_PREFIX, path.to_string_lossy())
 25:    }
 26:    
 27:    pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
 28:        let storage = get_storage()?;
 29:        let key = path_to_key(path.as_ref());
 30:        
 31:        let value = storage
 32:            .get_item(&key)
 33:            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to read from localStorage"))?
 34:            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "File not found"))?;
 35:        
 36:        // Decode from base64 to support binary data
 37:        base64::decode(&value)
 38:            .map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("Base64 decode error: {}", e)))
 39:    }
 40:    
 41:    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
 42:        let path = path.as_ref();
 43:        
 44:        // Create parent "directories" by just storing a marker
 45:        if let Some(parent) = path.parent() {
 46:            if !parent.as_os_str().is_empty() {
 47:                create_dir_all(parent)?;
 48:            }
 49:        }
 50:        
 51:        let storage = get_storage()?;
 52:        let key = path_to_key(path);
 53:        
 54:        // Encode as base64 to support binary data
 55:        let encoded = base64::encode(contents.as_ref());
 56:        
 57:        storage
 58:            .set_item(&key, &encoded)
 59:            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to write to localStorage"))
 60:    }
 61:    
 62:    pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
 63:        let storage = get_storage()?;
 64:        let key = format!("{}__dir__", path_to_key(path.as_ref()));
 65:        
 66:        // Just mark it as a directory
 67:        storage
 68:            .set_item(&key, "")
 69:            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to create directory marker"))?;
 70:        
 71:        Ok(())
 72:    }
 73:    
 74:    pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
 75:        let storage = get_storage()?;
 76:        let key = path_to_key(path.as_ref());
 77:        
 78:        storage
 79:            .remove_item(&key)
 80:            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to remove from localStorage"))?;
 81:        
 82:        Ok(())
 83:    }
 84:    
 85:    pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
 86:        let storage = get_storage()?;
 87:        let key = path_to_key(path.as_ref());
 88:        
 89:        let exists = storage
 90:            .get_item(&key)
 91:            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to check localStorage"))?
 92:            .is_some();
 93:        
 94:        Ok(exists)
 95:    }
 96:    
 97:    pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<BrowserMetadata> {
 98:        let storage = get_storage()?;
 99:        let key = path_to_key(path.as_ref());
100:        
101:        let value = storage
102:            .get_item(&key)
103:            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to read from localStorage"))?
104:            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "File not found"))?;
105:        
106:        // Decode to get actual size
107:        let decoded = base64::decode(&value)
108:            .map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("Base64 decode error: {}", e)))?;
109:        
110:        Ok(BrowserMetadata {
111:            len: decoded.len() as u64,
112:            is_file: !key.ends_with("__dir__"),
113:        })
114:    }
115:    
116:    pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
117:        let storage = get_storage()?;
118:        let prefix = path_to_key(path.as_ref());
119:        let prefix_with_slash = if prefix.is_empty() {
120:            STORAGE_PREFIX.to_string()
121:        } else {
122:            format!("{}/", prefix)
123:        };
124:        
125:        let mut entries = Vec::new();
126:        let len = storage
127:            .length()
128:            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to get storage length"))?;
129:        
130:        for i in 0..len {
131:            if let Ok(Some(key)) = storage.key(i) {
132:                if key.starts_with(&prefix_with_slash) {
133:                    let remainder = &key[prefix_with_slash.len()..];
134:                    // Only include direct children (not nested)
135:                    if !remainder.contains('/') && !remainder.ends_with("__dir__") {
136:                        entries.push(remainder.to_string());
137:                    }
138:                }
139:            }
140:        }
141:        
142:        Ok(entries)
143:    }
144:    
145:    // Simple metadata struct for browser
146:    pub struct BrowserMetadata {
147:        len: u64,
148:        is_file: bool,
149:    }
150:    
151:    impl BrowserMetadata {
152:        pub fn len(&self) -> u64 {
153:            self.len
154:        }
155:        
156:        pub fn is_file(&self) -> bool {
157:            self.is_file
158:        }
159:        
160:        pub fn is_dir(&self) -> bool {
161:            !self.is_file
162:        }
163:    }
164:}
165:
166:// === Native and WASI implementation ===
167:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
168:pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
169:    std::fs::read(path)
170:}
171:
172:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
173:pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
174:    let path = path.as_ref();
175:    
176:    // Ensure parent directory exists
177:    if let Some(parent) = path.parent() {
178:        if !parent.as_os_str().is_empty() {
179:            create_dir_all(parent)?;
180:        }
181:    }
182:    
183:    std::fs::write(path, contents)
184:}
185:
186:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
187:pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
188:    match std::fs::create_dir_all(path.as_ref()) {
189:        Ok(()) => Ok(()),
190:        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
191:        Err(e) => Err(e),
192:    }
193:}
194:
195:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
196:pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
197:    match std::fs::remove_file(path.as_ref()) {
198:        Ok(()) => Ok(()),
199:        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
200:        Err(e) => Err(e),
201:    }
202:}
203:
204:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
205:pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
206:    match std::fs::metadata(path.as_ref()) {
207:        Ok(_) => Ok(true),
208:        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
209:        Err(e) => Err(e),
210:    }
211:}
212:
213:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
214:pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<std::fs::Metadata> {
215:    std::fs::metadata(path)
216:}
217:
218:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
219:pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<std::fs::ReadDir> {
220:    std::fs::read_dir(path)
221:}
222:
223:// Re-export platform-specific types
224:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
225:pub use browser_fs::{BrowserMetadata as Metadata, read_dir};
226:
227:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
228:pub use browser_fs::{read, write, create_dir_all, remove_file, exists, metadata};
229:
230:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
231:pub use std::fs::{OpenOptions, File, Metadata, ReadDir, DirEntry};
232:
233:#[cfg(test)]
234:mod tests {
235:    use super::*;
236:    use std::sync::atomic::{AtomicU64, Ordering};
237:    
238:    // Counter to ensure unique test paths
239:    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
240:    
241:    /// Helper to get a platform-appropriate unique test path
242:    fn test_path(filename: &str) -> String {
243:        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
244:        let unique_name = format!("{}_{}", counter, filename);
245:        
246:        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
247:        {
248:            // Browser: Use simple paths (stored in localStorage)
249:            unique_name
250:        }
251:        
252:        #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
253:        {
254:            // WASI: Use relative paths in current directory
255:            unique_name
256:        }
257:        
258:        #[cfg(not(target_arch = "wasm32"))]
259:        {
260:            // Native: Use /tmp for test isolation
261:            format!("/tmp/ego2_test_{}", unique_name)
262:        }
263:    }
264:    
265:    #[test]
266:    fn test_write_and_read() {
267:        let path = test_path("write_read.txt");
268:        let content = b"Hello, platform!";
269:        
270:        // Write
271:        write(&path, content).expect("Failed to write file");
272:        
273:        // Read back
274:        let read_content = read(&path).expect("Failed to read file");
275:        assert_eq!(content, read_content.as_slice());
276:        
277:        // Cleanup
278:        remove_file(&path).ok();
279:    }
280:    
281:    #[test]
282:    fn test_exists() {
283:        let path = test_path("exists.txt");
284:        
285:        // Should not exist initially
286:        assert!(!exists(&path).expect("exists() check failed"));
287:        
288:        // Create it
289:        write(&path, b"test").expect("Failed to write file");
290:        
291:        // Now should exist
292:        assert!(exists(&path).expect("exists() check failed"));
293:        
294:        // Cleanup
295:        remove_file(&path).expect("Failed to remove file");
296:        
297:        // Should not exist after removal
298:        assert!(!exists(&path).expect("exists() check failed"));
299:    }
300:    
301:    #[test]
302:    fn test_remove_file_idempotent() {
303:        let path = test_path("remove.txt");
304:        
305:        // Removing non-existent file should succeed (idempotent behavior)
306:        remove_file(&path).expect("First removal should succeed even if file doesn't exist");
307:        
308:        // Create file
309:        write(&path, b"test").expect("Failed to write file");
310:        
311:        // Verify it exists
312:        assert!(exists(&path).expect("exists() check failed"));
313:        
314:        // Remove it
315:        remove_file(&path).expect("Failed to remove existing file");
316:        
317:        // Verify it's gone
318:        assert!(!exists(&path).expect("exists() check failed"));
319:        
320:        // Remove again - should still succeed (idempotent)
321:        remove_file(&path).expect("Second removal should succeed");
322:    }
323:    
324:    #[test]
325:    fn test_binary_content() {
326:        let path = test_path("binary.bin");
327:        
328:        // Write binary content (not valid UTF-8)
329:        let binary_data: Vec<u8> = (0..=255).collect();
330:        write(&path, &binary_data).expect("Failed to write binary data");
331:        
332:        // Read it back
333:        let read_data = read(&path).expect("Failed to read binary data");
334:        assert_eq!(binary_data, read_data);
335:        
336:        // Cleanup
337:        remove_file(&path).ok();
338:    }
339:    
340:    #[test]
341:    fn test_metadata() {
342:        let path = test_path("metadata.txt");
343:        let content = b"Hello, metadata!";
344:        
345:        // Write file
346:        write(&path, content).expect("Failed to write file");
347:        
348:        // Get metadata
349:        let meta = metadata(&path).expect("Failed to get metadata");
350:        
351:        // Verify it's a file
352:        assert!(meta.is_file());
353:        assert!(!meta.is_dir());
354:        
355:        // Verify size matches
356:        assert_eq!(meta.len(), content.len() as u64);
357:        
358:        // Cleanup
359:        remove_file(&path).ok();
360:    }
361:}
```


--- a/src/io.rs
+++ b/src/io.rs
``` rs
  1://! Platform-specific IO utilities.
  2://!
  3://! This module provides async stdin/stdout that work correctly across:
  4://! - Native platforms (Linux, macOS, Windows)
  5://! - WASI Preview 2 (wasm32-wasip2)
  6://! - Browser (wasm32-unknown-unknown)
  7://!
  8://! The key challenge on WASI P2 is that stdin.read() blocks the single-threaded
  9://! runtime, starving other async tasks (like timers). This implementation uses
 10://! WASI P2's native polling APIs to wait for stdin readiness without blocking.
 11:
 12:use std::io::Result;
 13:use std::pin::Pin;
 14:use std::task::{Context, Poll};
 15:use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
 16:
 17:// --- Public Interface ---
 18:
 19:pub use impl_platform::{stdin, stdout, Stdin, Stdout};
 20:
 21:// --- Native Implementation (Threaded) ---
 22:// Uses a background thread for blocking stdin reads, communicating via channel.
 23:#[cfg(not(target_arch = "wasm32"))]
 24:mod impl_platform {
 25:    use super::*;
 26:
 27:    pub struct Stdin {
 28:        receiver: tokio::sync::mpsc::Receiver<Result<Vec<u8>>>,
 29:        buffer: Vec<u8>,
 30:    }
 31:
 32:    pub struct Stdout {
 33:        inner: tokio::io::Stdout,
 34:    }
 35:
 36:    pub fn stdin() -> Stdin {
 37:        let (tx, rx) = tokio::sync::mpsc::channel(32);
 38:
 39:        // Native: Spawn a detached thread for blocking reads.
 40:        // This is necessary because std::io::Stdin::read() blocks,
 41:        // and we need the async runtime to remain responsive.
 42:        std::thread::spawn(move || {
 43:            let mut input = std::io::stdin();
 44:            let mut buf = [0u8; 1024];
 45:            use std::io::Read;
 46:
 47:            loop {
 48:                match input.read(&mut buf) {
 49:                    Ok(0) => break, // EOF
 50:                    Ok(n) => {
 51:                        if tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
 52:                            break; // Receiver dropped
 53:                        }
 54:                    }
 55:                    Err(e) => {
 56:                        let _ = tx.blocking_send(Err(e));
 57:                        break;
 58:                    }
 59:                }
 60:            }
 61:        });
 62:
 63:        Stdin {
 64:            receiver: rx,
 65:            buffer: Vec::new(),
 66:        }
 67:    }
 68:
 69:    pub fn stdout() -> Stdout {
 70:        Stdout {
 71:            inner: tokio::io::stdout(),
 72:        }
 73:    }
 74:
 75:    impl AsyncRead for Stdin {
 76:        fn poll_read(
 77:            mut self: Pin<&mut Self>,
 78:            cx: &mut Context<'_>,
 79:            buf: &mut ReadBuf<'_>,
 80:        ) -> Poll<Result<()>> {
 81:            loop {
 82:                // First, drain any buffered data
 83:                if !self.buffer.is_empty() {
 84:                    let len = std::cmp::min(buf.remaining(), self.buffer.len());
 85:                    buf.put_slice(&self.buffer[..len]);
 86:                    self.buffer.drain(..len);
 87:                    return Poll::Ready(Ok(()));
 88:                }
 89:
 90:                // Then try to receive more data from the background thread
 91:                match self.receiver.poll_recv(cx) {
 92:                    Poll::Ready(Some(Ok(chunk))) => {
 93:                        self.buffer = chunk;
 94:                        continue;
 95:                    }
 96:                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
 97:                    Poll::Ready(None) => return Poll::Ready(Ok(())), // Channel closed = EOF
 98:                    Poll::Pending => return Poll::Pending,
 99:                }
100:            }
101:        }
102:    }
103:
104:    impl AsyncWrite for Stdout {
105:        fn poll_write(
106:            mut self: Pin<&mut Self>,
107:            cx: &mut Context<'_>,
108:            buf: &[u8],
109:        ) -> Poll<Result<usize>> {
110:            Pin::new(&mut self.inner).poll_write(cx, buf)
111:        }
112:        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
113:            Pin::new(&mut self.inner).poll_flush(cx)
114:        }
115:        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
116:            Pin::new(&mut self.inner).poll_shutdown(cx)
117:        }
118:    }
119:}
120:
121:// --- WASI P2 Implementation ---
122:// 
123:// CRITICAL ARCHITECTURE NOTE:
124:// ==========================
125:// WASI P2 on a single-threaded tokio runtime presents a fundamental challenge:
126:// std::io::Stdin::read() BLOCKS the entire runtime until input arrives.
127:// This means timers, spawned tasks, and everything else gets starved.
128://
129:// The WASI P2 component model has non-blocking primitives (wasi:io/streams),
130:// but Rust's std::io doesn't expose them. The `wasi` crate provides bindings
131:// that let us use the native WASI P2 polling APIs.
132://
133:// SOLUTION: Timeout-based polling with wasi:io/poll
134:// =================================================
135:// We use wasi:io/poll with a very short timeout to check for stdin readiness.
136:// If data isn't ready, we yield back to tokio and re-poll later.
137:// This allows timers and other tasks to make progress between polls.
138://
139:// This approach has a tradeoff: slightly higher latency for stdin input
140:// (up to POLL_TIMEOUT_NS), but guarantees the runtime stays responsive.
141:#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
142:mod impl_platform {
143:    use super::*;
144:
145:    // How long to wait for stdin in each poll cycle (in nanoseconds).
146:    // Shorter = more responsive timers, but more CPU overhead.
147:    // 10ms is a good balance for interactive shells.
148:    const POLL_TIMEOUT_NS: u64 = 10_000_000; // 10ms
149:
150:    /// Async stdin for WASI P2 that cooperates with the tokio runtime.
151:    /// 
152:    /// Uses wasi:io/poll with short timeouts to avoid blocking the runtime.
153:    pub struct Stdin {
154:        buffer: Vec<u8>,
155:    }
156:
157:    pub struct Stdout;
158:
159:    pub fn stdin() -> Stdin {
160:        Stdin {
161:            buffer: Vec::new(),
162:        }
163:    }
164:
165:    pub fn stdout() -> Stdout {
166:        Stdout
167:    }
168:
169:    impl AsyncRead for Stdin {
170:        fn poll_read(
171:            mut self: Pin<&mut Self>,
172:            cx: &mut Context<'_>,
173:            buf: &mut ReadBuf<'_>,
174:        ) -> Poll<Result<()>> {
175:            // First, drain any buffered data from previous reads
176:            if !self.buffer.is_empty() {
177:                let len = std::cmp::min(buf.remaining(), self.buffer.len());
178:                buf.put_slice(&self.buffer[..len]);
179:                self.buffer.drain(..len);
180:                return Poll::Ready(Ok(()));
181:            }
182:
183:            // Get stdin stream - note: in WASI P2, get_stdin() returns a fresh handle each time
184:            let stream = wasi::cli::stdin::get_stdin();
185:            
186:            // Get a pollable for the stdin stream
187:            let stdin_pollable = stream.subscribe();
188:            
189:            // Also create a timer pollable for our timeout
190:            let timer_pollable = wasi::clocks::monotonic_clock::subscribe_duration(POLL_TIMEOUT_NS);
191:            
192:            // Poll both: stdin readiness OR timeout
193:            // This is the key: poll() will return when EITHER is ready,
194:            // so we won't block forever waiting for stdin.
195:            let ready_indices = wasi::io::poll::poll(&[&stdin_pollable, &timer_pollable]);
196:            
197:            // Check if stdin is ready (index 0)
198:            let stdin_ready = ready_indices.iter().any(|&i| i == 0);
199:            
200:            if stdin_ready {
201:                // Stdin has data! Read it non-blocking.
202:                // WASI streams return whatever is available (may be less than requested).
203:                match stream.read(buf.remaining() as u64) {
204:                    Ok(bytes) => {
205:                        if bytes.is_empty() {
206:                            // EOF
207:                            Poll::Ready(Ok(()))
208:                        } else {
209:                            buf.put_slice(&bytes);
210:                            Poll::Ready(Ok(()))
211:                        }
212:                    }
213:                    Err(wasi::io::streams::StreamError::Closed) => {
214:                        // Stream closed = EOF
215:                        Poll::Ready(Ok(()))
216:                    }
217:                    Err(_e) => {
218:                        Poll::Ready(Err(std::io::Error::new(
219:                            std::io::ErrorKind::Other,
220:                            "WASI stream read error",
221:                        )))
222:                    }
223:                }
224:            } else {
225:                // Timeout fired, stdin not ready.
226:                // Yield back to tokio so other tasks can run.
227:                cx.waker().wake_by_ref();
228:                Poll::Pending
229:            }
230:        }
231:    }
232:
233:    impl AsyncWrite for Stdout {
234:        fn poll_write(
235:            self: Pin<&mut Self>,
236:            _cx: &mut Context<'_>,
237:            buf: &[u8],
238:        ) -> Poll<Result<usize>> {
239:            // Get stdout stream
240:            let stream = wasi::cli::stdout::get_stdout();
241:            
242:            // Check how much we can write without blocking
243:            match stream.check_write() {
244:                Ok(0) => {
245:                    // Can't write right now, would need to wait
246:                    // For simplicity, just report we wrote 0 bytes
247:                    Poll::Ready(Ok(0))
248:                }
249:                Ok(n) => {
250:                    let to_write = std::cmp::min(n as usize, buf.len());
251:                    match stream.write(&buf[..to_write]) {
252:                        Ok(()) => Poll::Ready(Ok(to_write)),
253:                        Err(_) => Poll::Ready(Err(std::io::Error::new(
254:                            std::io::ErrorKind::Other,
255:                            "WASI stream write error",
256:                        ))),
257:                    }
258:                }
259:                Err(_) => Poll::Ready(Err(std::io::Error::new(
260:                    std::io::ErrorKind::Other,
261:                    "WASI stream check_write error",
262:                ))),
263:            }
264:        }
265:
266:        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
267:            let stream = wasi::cli::stdout::get_stdout();
268:            
269:            // flush() is non-blocking, just requests a flush.
270:            // blocking_flush() will wait for it to complete.
271:            match stream.flush() {
272:                Ok(()) => {
273:                    match stream.blocking_flush() {
274:                        Ok(()) => Poll::Ready(Ok(())),
275:                        Err(_) => Poll::Ready(Err(std::io::Error::new(
276:                            std::io::ErrorKind::Other,
277:                            "WASI stream flush error",
278:                        ))),
279:                    }
280:                }
281:                Err(_) => Poll::Ready(Err(std::io::Error::new(
282:                    std::io::ErrorKind::Other,
283:                    "WASI stream flush error",
284:                ))),
285:            }
286:        }
287:
288:        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
289:            Poll::Ready(Ok(()))
290:        }
291:    }
292:}
293:
294:// --- Browser Implementation (Stubs) ---
295:// Browser doesn't have traditional stdin/stdout. These are no-ops.
296:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
297:mod impl_platform {
298:    use super::*;
299:
300:    pub struct Stdin {}
301:    pub struct Stdout {}
302:
303:    pub fn stdin() -> Stdin {
304:        Stdin {}
305:    }
306:    pub fn stdout() -> Stdout {
307:        Stdout {}
308:    }
309:
310:    impl AsyncRead for Stdin {
311:        fn poll_read(
312:            self: Pin<&mut Self>,
313:            _cx: &mut Context<'_>,
314:            _buf: &mut ReadBuf<'_>,
315:        ) -> Poll<Result<()>> {
316:            // Browser stdin is always EOF
317:            Poll::Ready(Ok(()))
318:        }
319:    }
320:
321:    impl AsyncWrite for Stdout {
322:        fn poll_write(
323:            self: Pin<&mut Self>,
324:            _cx: &mut Context<'_>,
325:            buf: &[u8],
326:        ) -> Poll<Result<usize>> {
327:            // Pretend we wrote everything (browser uses console.log instead)
328:            Poll::Ready(Ok(buf.len()))
329:        }
330:        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
331:            Poll::Ready(Ok(()))
332:        }
333:        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
334:            Poll::Ready(Ok(()))
335:        }
336:    }
337:}
338:
339:#[cfg(test)]
340:mod tests {
341:    use super::*;
342:
343:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
344:    use tokio::io::AsyncReadExt;
345:
346:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
347:    #[wasm_bindgen_test::wasm_bindgen_test]
348:    async fn test_stdin_browser_noop() {
349:        let mut stdin = stdin();
350:        let mut buf = [0u8; 10];
351:        let n = stdin.read(&mut buf).await.unwrap();
352:        assert_eq!(n, 0);
353:    }
354:
355:    #[test]
356:    fn test_io_creation() {
357:        let _stdin = stdin();
358:        let _stdout = stdout();
359:    }
360:}
```


--- a/src/lib.rs
+++ b/src/lib.rs
``` rs
  1://! Platform abstraction layer for Rust applications targeting native, WASI, and browser environments.
  2://!
  3://! This library provides a unified API for common operations across different platforms:
  4://! - **Logging**: Platform-appropriate logging initialization
  5://! - **Async spawn**: Task spawning with correct bounds for each platform
  6://! - **Time**: Sleep, intervals, and system time
  7://! - **Sync**: Broadcast channels (native only, stub on WASM)
  8://!
  9://! # Platform Detection
 10://!
 11://! ```
 12://! use aloeplatform::{Platform, detect};
 13://!
 14://! match detect() {
 15://!     Platform::Native => println!("Running on native platform"),
 16://!     Platform::Wasi => println!("Running on WASI"),
 17://!     Platform::Browser => println!("Running in browser"),
 18://! }
 19://! ```
 20:
 21:pub mod logging;
 22:pub mod spawn;
 23:pub mod sync;
 24:pub mod time;
 25:pub mod io;
 26:pub mod fs;
 27:
 28:pub use spawn::{spawn, TaskHandle};
 29:pub use time::{sleep, Interval, SystemTime, UNIX_EPOCH, Instant};
 30:pub use io::stdin;
 31:pub use sync::broadcast;
 32:
 33:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
 34:pub enum Platform {
 35:    Native,
 36:    Wasi,
 37:    Browser,
 38:}
 39:
 40:pub fn detect() -> Platform {
 41:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 42:    return Platform::Browser;
 43:
 44:    #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
 45:    return Platform::Wasi;
 46:
 47:    #[cfg(not(target_arch = "wasm32"))]
 48:    return Platform::Native;
 49:}
 50:
 51:
 52:pub use logging::register_output_hook;
 53:pub fn init() {
 54:    println!("[aloeplatform lib.rs] init");
 55:    logging::init();
 56:}
 57:
 58:// Backward compatibility alias
 59:pub use time::sleep as wait_for_timeout;
 60:
 61:#[cfg(test)]
 62:mod tests {
 63:    use super::*;
 64:
 65:    #[test]
 66:    fn test_platform_detection() {
 67:        let platform = detect();
 68:        
 69:        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 70:        assert_eq!(platform, Platform::Browser);
 71:
 72:        #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
 73:        assert_eq!(platform, Platform::Wasi);
 74:
 75:        #[cfg(not(target_arch = "wasm32"))]
 76:        assert_eq!(platform, Platform::Native);
 77:    }
 78:
 79:    #[test]
 80:    fn test_platform_debug() {
 81:        let platform = detect();
 82:        let debug_str = format!("{:?}", platform);
 83:        assert!(debug_str.len() > 0);
 84:    }
 85:
 86:    #[test]
 87:    fn test_platform_equality() {
 88:        let p1 = detect();
 89:        let p2 = detect();
 90:        assert_eq!(p1, p2);
 91:        
 92:        assert_eq!(Platform::Native, Platform::Native);
 93:        assert_eq!(Platform::Wasi, Platform::Wasi);
 94:        assert_eq!(Platform::Browser, Platform::Browser);
 95:        
 96:        assert_ne!(Platform::Native, Platform::Wasi);
 97:        assert_ne!(Platform::Native, Platform::Browser);
 98:        assert_ne!(Platform::Wasi, Platform::Browser);
 99:    }
100:
101:    #[test]
102:    fn test_init_does_not_panic() {
103:        // Init should be idempotent and not panic
104:        init();
105:    }
106:}
```


--- a/src/logging.rs
+++ b/src/logging.rs
``` rs
  1://! Platform-specific logging initialization.
  2:use std::sync::RwLock;
  3:use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
  4:
  5:// Track last output time - logs will update this
  6:pub static LAST_OUTPUT_TIME: AtomicU64 = AtomicU64::new(0);
  7:
  8:// Global hook for custom output behavior (e.g. shell prompt rewriting)
  9:// We use a static RwLock to allow safe concurrent access and modification.
 10:type LogHook = Box<dyn Fn(&log::Record) + Sync + Send>;
 11:static OUTPUT_HOOK: RwLock<Option<LogHook>> = RwLock::new(None);
 12:
 13:pub fn register_output_hook<F>(hook: F)
 14:where
 15:    F: Fn(&log::Record) + Sync + Send + 'static,
 16:{
 17:    if let Ok(mut lock) = OUTPUT_HOOK.write() {
 18:        *lock = Some(Box::new(hook));
 19:    }
 20:}
 21:
 22:pub fn notify_output() {
 23:    // use std::time::{SystemTime, UNIX_EPOCH};
 24:    let now = crate::SystemTime::now()
 25:        .duration_since(crate::SystemTime::UNIX_EPOCH)
 26:        .unwrap()
 27:        .as_millis() as u64;
 28:    LAST_OUTPUT_TIME.store(now, Ordering::Relaxed);
 29:}
 30:
 31:// Track if we're in command mode (WASI P2 mostly, but available generally)
 32:static COMMAND_MODE: AtomicBool = AtomicBool::new(false);
 33:
 34:static BACKEND_LOGGER: RwLock<Option<Box<dyn log::Log + Send + Sync>>> = RwLock::new(None);
 35:
 36:pub fn set_command_mode(enabled: bool) {
 37:    COMMAND_MODE.store(enabled, Ordering::Relaxed);
 38:}
 39:
 40:struct SimpleLogger;
 41:
 42:impl log::Log for SimpleLogger {
 43:    fn enabled(&self, metadata: &log::Metadata) -> bool {
 44:        // Delegate enabled check to backend if present
 45:        if let Ok(guard) = BACKEND_LOGGER.read() {
 46:            if let Some(logger) = guard.as_ref() {
 47:                return logger.enabled(metadata);
 48:            }
 49:        }
 50:        metadata.level() <= log::Level::Info
 51:    }
 52:
 53:    fn log(&self, record: &log::Record) {
 54:        if !self.enabled(record.metadata()) {
 55:            return;
 56:        }
 57:
 58:        // 1. Update activity timestamp
 59:        notify_output();
 60:
 61:        // 2. Check Command Mode
 62:        // We bypass command mode blocking if the target is explicit "term" output
 63:        let is_term_output = record.target() == "term";
 64:        if !is_term_output && COMMAND_MODE.load(Ordering::Relaxed) {
 65:            return;
 66:        }
 67:
 68:        // 3. Dispatch to Hook (if present)
 69:        // Hooks take precedence over backend output for shell integration
 70:        if let Ok(guard) = OUTPUT_HOOK.read() {
 71:            if let Some(hook) = guard.as_ref() {
 72:                hook(record);
 73:                return;
 74:            }
 75:        }
 76:
 77:        // 4. Default Fallback (Delegate to Backend)
 78:        if let Ok(guard) = BACKEND_LOGGER.read() {
 79:            if let Some(logger) = guard.as_ref() {
 80:                logger.log(record);
 81:                return;
 82:            }
 83:        }
 84:        
 85:        // 5. Ultimate Fallback (if backend missing)
 86:        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 87:        {
 88:            use wasm_bindgen::JsValue;
 89:            let msg = format!("[{}] {}", record.level(), record.args());
 90:            web_sys::console::log_1(&JsValue::from_str(&msg));
 91:        }
 92:        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 93:        {
 94:            println!("[{}] {}", record.level(), record.args());
 95:        }
 96:    }
 97:
 98:    fn flush(&self) {
 99:        if let Ok(guard) = BACKEND_LOGGER.read() {
100:            if let Some(logger) = guard.as_ref() {
101:                logger.flush();
102:            }
103:        }
104:    }
105:}
106:
107:static LOGGER: SimpleLogger = SimpleLogger;
108:
109:/// Initialize logging for the current platform.
110:pub fn init() {
111:    // 1. Configure Platform Backend
112:    let mut max_level = log::LevelFilter::Info;
113:
114:    // A. Web (console_log)
115:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
116:    {
117:        // Use console_log crate to map generic Log calls to console.debug/info/warn
118:        // This makes browser filtering work natively.
119:        // We initialize it manually (not via init()) to wrap it.
120:        // Note: The struct is named `WebConsoleLogger` in newer versions or exposed differently.
121:        // If manual instantiation is tricky, we can implement a trivial wrapper that calls web_sys::console.
122:        // But let's try the suggestion from the compiler first if available, otherwise fallback.
123:        
124:        struct WebLogger;
125:        impl log::Log for WebLogger {
126:            fn enabled(&self, _metadata: &log::Metadata) -> bool { true }
127:            fn log(&self, record: &log::Record) {
128:                 // Map Rust log levels to console methods
129:                 use wasm_bindgen::JsValue;
130:                 let msg = format!("{}", record.args());
131:                 let js_msg = JsValue::from_str(&msg);
132:                 
133:                 match record.level() {
134:                     log::Level::Error => web_sys::console::error_1(&js_msg),
135:                     log::Level::Warn => web_sys::console::warn_1(&js_msg),
136:                     log::Level::Info => web_sys::console::info_1(&js_msg),
137:                     log::Level::Debug => web_sys::console::debug_1(&js_msg),
138:                     log::Level::Trace => web_sys::console::trace_1(&js_msg),
139:                 }
140:            }
141:            fn flush(&self) {}
142:        }
143:        
144:        let logger = WebLogger;
145:        max_level = log::LevelFilter::Debug; // Let browser filter
146:        if let Ok(mut guard) = BACKEND_LOGGER.write() {
147:            *guard = Some(Box::new(logger));
148:        }
149:        console_error_panic_hook::set_once();
150:    }
151:
152:    // B. Native & WASI (env_logger)
153:    // env_logger works on WASI too, reading RUST_LOG from the host environment.
154:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
155:    {
156:        // Default to INFO if RUST_LOG is not set
157:        let env = env_logger::Env::default().default_filter_or("info");
158:        let logger = env_logger::Builder::from_env(env).build();
159:        
160:        max_level = logger.filter();
161:        if let Ok(mut guard) = BACKEND_LOGGER.write() {
162:            *guard = Some(Box::new(logger));
163:        }
164:    }
165:
166:    // 2. Install SimpleLogger as the global subscriber
167:    if log::set_logger(&LOGGER).is_ok() {
168:        log::set_max_level(max_level);
169:    }
170:}
171:
172:#[cfg(test)]
173:mod tests {
174:    use super::*;
175:
176:    #[test]
177:    fn test_logging_init() {
178:        // Should not panic
179:        init();
180:
181:        // Test that we can log after initialization
182:        log::info!("Test log message");
183:        log::debug!("Debug message");
184:        log::warn!("Warning message");
185:    }
186:
187:    #[test]
188:    fn test_logging_levels() {
189:        init();
190:
191:        // These should all work without panicking
192:        log::error!("Error level");
193:        log::warn!("Warn level");
194:        log::info!("Info level");
195:        log::debug!("Debug level");
196:        log::trace!("Trace level");
197:    }
198:}
```


--- a/src/main.rs
+++ b/src/main.rs
``` rs
 1:use aloeplatform::{detect, init, spawn, sleep, Interval, Platform};
 2:use std::time::Duration;
 3:
 4:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 5:use wasm_bindgen::prelude::*;
 6:
 7:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 8:#[cfg_attr(not(target_arch = "wasm32"), tokio::main(flavor = "multi_thread"))]
 9:#[cfg_attr(target_arch = "wasm32", tokio::main(flavor = "current_thread"))]
10:async fn main() {
11:    run().await;
12:}
13:
14:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
15:fn main() {
16:    // Browser: spawn_local requires setup through wasm-bindgen
17:    wasm_bindgen_futures::spawn_local(run());
18:}
19:
20:async fn run() {
21:    // Initialize the platform
22:    init();
23:
24:    // Detect and log the platform
25:    let platform = detect();
26:    log::info!("Running on platform: {:?}", platform);
27:
28:    match platform {
29:        Platform::Native => {
30:            log::info!("Native platform detected - full tokio runtime available");
31:        }
32:        Platform::Wasi => {
33:            log::info!("WASI platform detected - tokio runtime with limited features");
34:        }
35:        Platform::Browser => {
36:            log::info!("Browser platform detected - using wasm_bindgen_futures");
37:        }
38:    }
39:
40:    // Example: Spawn a background task
41:    spawn(async {
42:        log::info!("Background task started");
43:        sleep(Duration::from_millis(100)).await;
44:        log::info!("Background task completed");
45:    });
46:
47:    // Example: Use sleep
48:    log::info!("Sleeping for 200ms...");
49:    sleep(Duration::from_millis(200)).await;
50:    log::info!("Sleep completed");
51:
52:    // Example: Use interval
53:    log::info!("Starting interval (3 ticks)...");
54:    let mut interval = Interval::new(Duration::from_millis(100));
55:    for i in 0..3 {
56:        interval.tick().await;
57:        log::info!("Interval tick {}", i + 1);
58:    }
59:
60:    log::info!("Example completed successfully!");
61:}
```


--- a/src/spawn.rs
+++ b/src/spawn.rs
``` rs
  1://! Platform-specific task spawning.
  2:
  3:use std::future::Future;
  4:use tokio::sync::oneshot;
  5:use std::sync::atomic::{AtomicBool, Ordering};
  6:use std::sync::Arc;
  7:
  8:use std::pin::Pin;
  9:use std::task::{Context, Poll};
 10:
 11:/// Spawn a future onto the appropriate runtime for the current platform.
 12:///
 13:/// - **Browser**: Uses `wasm_bindgen_futures::spawn_local` (no Send required)
 14:/// - **Native/WASI**: Uses `tokio::spawn` (requires Send)
 15:///
 16:/// # Examples
 17:///
 18:/// ```no_run
 19:/// use aloeplatform::spawn;
 20:///
 21:/// spawn(async {
 22:///     println!("Running in background");
 23:/// });
 24:/// ```
 25:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 26:pub fn spawn<F, T>(future: F) -> TaskHandle<T>
 27:where
 28:    F: Future<Output = T> + 'static,
 29:    T: Send + 'static,
 30:{
 31:    let is_done = Arc::new(AtomicBool::new(false));
 32:    let is_done_clone = is_done.clone();
 33:
 34:    let (tx, rx) = oneshot::channel();
 35:    wasm_bindgen_futures::spawn_local(async move {
 36:        let result = future.await;
 37:        is_done_clone.store(true, Ordering::Release);
 38:        let _ = tx.send(result);
 39:    });
 40:    TaskHandle { rx, is_done }
 41:}
 42:
 43:/// Spawn a future onto the appropriate runtime for the current platform.
 44:///
 45:/// - **Browser**: Uses `wasm_bindgen_futures::spawn_local` (no Send required)
 46:/// - **Native/WASI**: Uses `tokio::spawn` (requires Send)
 47:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 48:pub fn spawn<F, T>(future: F) -> TaskHandle<T>
 49:where
 50:    F: Future<Output = T> + Send + 'static,
 51:    T: Send + 'static,
 52:{
 53:    let is_done = Arc::new(AtomicBool::new(false));
 54:    let is_done_clone = is_done.clone();
 55:
 56:    let (tx, rx) = oneshot::channel();
 57:    let inner = tokio::spawn(async move {
 58:        let result = future.await;
 59:        is_done_clone.store(true, Ordering::Release);
 60:        let _ = tx.send(result);
 61:    });
 62:    TaskHandle { rx, inner, is_done }
 63:}
 64:
 65:
 66:pub struct TaskHandle<T> {
 67:    rx: oneshot::Receiver<T>,
 68:    // On native, we keep the actual JoinHandle to allow forceful aborts
 69:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 70:    inner: tokio::task::JoinHandle<()>,
 71:
 72:    // Valid on all platforms
 73:    is_done: Arc<AtomicBool>,
 74:}
 75:
 76:impl<T> TaskHandle<T> {
 77:    /// Await the completion of the task and return the result.
 78:    pub async fn join(self) -> Result<T, oneshot::error::RecvError> {
 79:        self.rx.await
 80:    }
 81:
 82:    /// Forcefully terminate the task (Native only).
 83:    pub fn abort(&self) {
 84:        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 85:        self.inner.abort();
 86:        // Note: Browser/spawn_local does not support native aborting.
 87:    }
 88:
 89:    /// Returns `true` if the task has finished.
 90:    ///
 91:    /// # Platform Behavior
 92:    /// - **Native**: Returns the accurate state from the Tokio JoinHandle.
 93:    /// - **WASM**: Always returns `false` (limitation of `spawn_local`).
 94:    pub fn is_finished(&self) -> bool {
 95:        // CASE A: Native & WASI (Use the real handle)
 96:        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 97:        {
 98:            return self.inner.is_finished();
 99:        }
100:
101:        // CASE B: Browser / Unknown OS (Use the manual flag)
102:        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
103:        {
104:            return self.is_done.load(Ordering::Acquire);
105:        }
106:    }
107:}
108:
109:// Allow the handle to be awaited directly!
110:impl<T> Future for TaskHandle<T> {
111:    type Output = Result<T, oneshot::error::RecvError>;
112:
113:    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
114:        // We just proxy the poll call to the inner receiver (rx)
115:        Pin::new(&mut self.rx).poll(cx)
116:    }
117:}
118:
119:#[cfg(test)]
120:mod tests {
121:    use super::*;
122:    use std::sync::Arc;
123:    use std::sync::atomic::{AtomicBool, Ordering};
124:
125:    #[tokio::test]
126:    async fn test_spawn_with_handle() {
127:        let handle = spawn(async { 42 });
128:
129:        let result = handle.join().await.unwrap();
130:        assert_eq!(result, 42);
131:    }
132:
133:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
134:    #[tokio::test]
135:    async fn test_spawn_executes() {
136:        let flag = Arc::new(AtomicBool::new(false));
137:        let flag_clone = flag.clone();
138:
139:        spawn(async move {
140:            flag_clone.store(true, Ordering::SeqCst);
141:        });
142:
143:        // Give the spawned task time to execute
144:        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
145:
146:        assert!(flag.load(Ordering::SeqCst));
147:    }
148:
149:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
150:    #[tokio::test]
151:    async fn test_spawn_multiple_tasks() {
152:        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
153:
154:        for _ in 0..10 {
155:            let counter_clone = counter.clone();
156:            spawn(async move {
157:                counter_clone.fetch_add(1, Ordering::SeqCst);
158:            });
159:        }
160:
161:        // Give tasks time to execute
162:        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
163:
164:        assert_eq!(counter.load(Ordering::SeqCst), 10);
165:    }
166:
167:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
168:    #[wasm_bindgen_test::wasm_bindgen_test]
169:    async fn test_spawn_browser() {
170:        let flag = Arc::new(AtomicBool::new(false));
171:        let flag_clone = flag.clone();
172:
173:        spawn(async move {
174:            flag_clone.store(true, Ordering::SeqCst);
175:        });
176:
177:        // Give the spawned task time to execute
178:        gloo_timers::future::sleep(std::time::Duration::from_millis(50)).await;
179:
180:        assert!(flag.load(Ordering::SeqCst));
181:    }
182:}
```


--- a/src/sync.rs
+++ b/src/sync.rs
``` rs
 1:#[deprecated(note = "Just wraps tokio::sync::broadcast, probably doesn't do that good a job of it")]
 2:pub mod broadcast {
 3:    use tokio::sync::broadcast as tokio_broadcast;
 4:    pub use tokio_broadcast::error;
 5:
 6:    #[derive(Clone)]
 7:    pub struct Sender<T> {
 8:        inner: tokio_broadcast::Sender<T>,
 9:    }
10:
11:    pub struct Receiver<T> {
12:        inner: tokio_broadcast::Receiver<T>,
13:    }
14:
15:    pub fn channel<T: Clone>(capacity: usize) -> (Sender<T>, Receiver<T>) {
16:        let (tx, rx) = tokio_broadcast::channel(capacity);
17:        (Sender { inner: tx }, Receiver { inner: rx })
18:    }
19:
20:    impl<T: Clone> Sender<T> {
21:        pub fn send(&self, value: T) -> Result<usize, ()> {
22:            // In the browser, this won't actually "broadcast" across threads 
23:            // (since there is only one), but it will broadcast to all 
24:            // async tasks listening to this bus.
25:            self.inner.send(value).map_err(|_| ())
26:        }
27:
28:        pub fn subscribe(&self) -> Receiver<T> {
29:            Receiver { inner: self.inner.subscribe() }
30:        }
31:    }
32:
33:    impl<T: Clone> Receiver<T> {
34:        pub async fn recv(&mut self) -> Result<T, tokio_broadcast::error::RecvError> {
35:            self.inner.recv().await
36:        }
37:
38:        pub fn resubscribe(&self) -> Self {
39:            Self { inner: self.inner.resubscribe() }
40:        }
41:    }
42:}
```


--- a/src/time.rs
+++ b/src/time.rs
``` rs
  1://! Platform-specific time utilities.
  2:
  3:use std::time::Duration;
  4:
  5:// --- SystemTime ---
  6:
  7:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
  8:mod system_time {
  9:    pub use instant::SystemTime;
 10:    pub const UNIX_EPOCH: SystemTime = SystemTime::UNIX_EPOCH;
 11:}
 12:
 13:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 14:mod system_time {
 15:    pub use std::time::{SystemTime, UNIX_EPOCH};
 16:}
 17:
 18:pub use system_time::*;
 19:
 20:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 21:pub use instant::Instant;
 22:
 23:#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 24:pub use std::time::Instant;
 25:
 26:/// Asynchronously  for the specified duration.
 27:///
 28:/// Uses the appropriate  implementation for the current platform:
 29:/// - **Native/WASI**: `tokio::time::`
 30:/// - **Browser**: `gloo_timers::future::`
 31:///
 32:/// # Examples
 33:///
 34:/// ```no_run
 35:/// use std::time::Duration;
 36:///
 37:/// # async {
 38:/// (Duration::from_secs(1));
 39:/// # };
 40:/// ```
 41:pub async fn sleep(duration: Duration) {
 42:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 43:    {
 44:        tokio::time::sleep(duration).await;
 45:    }
 46:
 47:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 48:    {
 49:        // wrapper implements Send for the !Send gloo future.
 50:        struct SendFuture<F>(F);
 51:
 52:        // SAFETY: WASM is single-threaded. We aren't actually sending this across threads.
 53:        unsafe impl<F> Send for SendFuture<F> {}
 54:
 55:        impl<F: std::future::Future> std::future::Future for SendFuture<F> {
 56:            type Output = F::Output;
 57:            fn poll(
 58:                self: std::pin::Pin<&mut Self>,
 59:                cx: &mut std::task::Context<'_>,
 60:            ) -> std::task::Poll<Self::Output> {
 61:                // Project Pin<&mut Wrapper> to Pin<&mut Inner>
 62:                unsafe { self.map_unchecked_mut(|s| &mut s.0).poll(cx) }
 63:            }
 64:        }
 65:
 66:        SendFuture(gloo_timers::future::sleep(duration)).await;
 67:    }
 68:}
 69:
 70:// --- Interval ---
 71:
 72:/// Defines the behavior of an `Interval` when it misses a tick.
 73:///
 74:/// This mirrors `tokio::time::MissedTickBehavior`.
 75:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
 76:pub enum MissedTickBehavior {
 77:    /// Ticks happen as fast as possible until caught up.
 78:    Burst,
 79:    /// Ticks usually happen after the specified duration, but will "skip"
 80:    /// missed ticks to prevent burstiness, resetting the schedule to the current time.
 81:    Skip,
 82:    /// The interval is effectively restarted. The next tick will happen
 83:    /// `duration` after the current tick completes.
 84:    Delay,
 85:}
 86:
 87:/// A periodic timer that ticks at a fixed interval.
 88:///
 89:/// Uses the appropriate interval implementation for the current platform:
 90:/// - **Native/WASI**: `tokio::time::Interval`
 91:/// - **Browser**: Manual sleep-based implementation
 92:pub struct Interval {
 93:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
 94:    state: BrowserIntervalState,
 95:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
 96:    inner: tokio::time::Interval,
 97:}
 98:
 99:#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
100:struct BrowserIntervalState {
101:    duration: Duration,
102:    next_tick: Option<instant::Instant>, // None = Not started yet
103:    behavior: MissedTickBehavior,
104:}
105:
106:// SAFETY: On Native, we wrap tokio::time::Interval which is thread-safe.
107:// On Browser, we use a Duration which is a primitive. We manually implement
108:// Send to satisfy the Service harness's global requirements.
109:unsafe impl Send for Interval {}
110:unsafe impl Sync for Interval {}
111:
112:impl Interval {
113:    /// Create a new interval with the specified duration.
114:    pub fn new(duration: Duration) -> Self {
115:        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
116:        {
117:            Self {
118:                inner: tokio::time::interval(duration),
119:            }
120:        }
121:
122:        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
123:        {
124:            Self {
125:                state: BrowserIntervalState {
126:                    duration,
127:                    next_tick: None,
128:                    behavior: MissedTickBehavior::Burst, // Default to Burst to match Tokio
129:                },
130:            }
131:        }
132:    }
133:
134:    /// Configures the behavior for missed ticks.
135:    pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
136:        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
137:        {
138:            let tokio_behavior = match behavior {
139:                MissedTickBehavior::Burst => tokio::time::MissedTickBehavior::Burst,
140:                MissedTickBehavior::Skip => tokio::time::MissedTickBehavior::Skip,
141:                MissedTickBehavior::Delay => tokio::time::MissedTickBehavior::Delay,
142:            };
143:            self.inner.set_missed_tick_behavior(tokio_behavior);
144:        }
145:
146:        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
147:        {
148:            self.state.behavior = behavior;
149:        }
150:    }
151:
152:    /// Wait for the next tick of the interval.
153:    pub async fn tick(&mut self) {
154:        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
155:        {
156:            self.inner.tick().await;
157:        }
158:
159:        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
160:        {
161:            let now = instant::Instant::now();
162:
163:            // 1. Initialize logic (First tick fires immediately)
164:            let next_tick = match self.state.next_tick {
165:                Some(t) => t,
166:                None => {
167:                    self.state.next_tick = Some(now + self.state.duration);
168:                    return;
169:                }
170:            };
171:
172:            // 2. Schedule logic
173:            if now < next_tick {
174:                // We are early (normal case). Wait for the deadline.
175:                sleep(next_tick - now).await;
176:                self.state.next_tick = Some(next_tick + self.state.duration);
177:            } else {
178:                // We are late (Missed Tick)
179:                match self.state.behavior {
180:                    MissedTickBehavior::Burst => {
181:                        // Catch up mode: Schedule relative to the OLD deadline.
182:                        // We return immediately to "fire" this delayed tick.
183:                        self.state.next_tick = Some(next_tick + self.state.duration);
184:                    }
185:                    MissedTickBehavior::Skip => {
186:                        // Skip mode: Abandon old schedule, reset to NOW + Duration.
187:                        // We return immediately for *this* tick, but the *next* one is pushed back.
188:                        self.state.next_tick = Some(now + self.state.duration);
189:                    }
190:                    MissedTickBehavior::Delay => {
191:                        // Delay mode: Wait full duration starting NOW.
192:                        // This introduces drift.
193:                        sleep(self.state.duration).await;
194:                        self.state.next_tick = Some(instant::Instant::now() + self.state.duration);
195:                    }
196:                }
197:            }
198:        }
199:    }
200:}
201:
202:
203:
204:// --- Error Type ---
205:
206:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
207:pub struct Elapsed;
208:
209:impl std::fmt::Display for Elapsed {
210:    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
211:        write!(f, "deadline has elapsed")
212:    }
213:}
214:
215:impl std::error::Error for Elapsed {}
216:
217:// --- Timeout Implementation ---
218:
219:/// Require a `Future` to complete before the specified duration has elapsed.
220:///
221:/// If the future completes before the duration has elapsed, then the completed
222:/// value is returned. Otherwise, an error is returned and the future is
223:/// canceled.
224:pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, Elapsed>
225:where
226:    F: Future,
227:{
228:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
229:    {
230:        // Native/WASI: Wrapper around Tokio
231:        match tokio::time::timeout(duration, future).await {
232:            Ok(val) => Ok(val),
233:            Err(_) => Err(Elapsed),
234:        }
235:    }
236:
237:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
238:    {
239:        use futures::future::{select, Either};
240:        use gloo_timers::future::TimeoutFuture;
241:        
242:        // 1. Create the sleep future (The "Bomb")
243:        let delay = TimeoutFuture::new(duration.as_millis() as u32);
244:        
245:        // 2. Pin them both (Select requires pinning)
246:        futures::pin_mut!(future);
247:        futures::pin_mut!(delay);
248:
249:        // 3. Race them!
250:        match select(future, delay).await {
251:            Either::Left((val, _)) => Ok(val), // Future finished first
252:            Either::Right(_) => Err(Elapsed),  // Timer finished first
253:        }
254:    }
255:}
256:
257:
258:
259:
260:
261:
262:
263:
264:#[cfg(test)]
265:mod tests {
266:    use super::*;
267:
268:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
269:    #[tokio::test]
270:    async fn test_sleep() {
271:        let start = std::time::Instant::now();
272:        sleep(Duration::from_millis(100)).await;
273:        let elapsed = start.elapsed();
274:
275:        // Should sleep for at least 100ms (with some tolerance)
276:        assert!(elapsed >= Duration::from_millis(90));
277:    }
278:
279:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
280:    #[wasm_bindgen_test::wasm_bindgen_test]
281:    async fn test_sleep_browser() {
282:        let start = instant::Instant::now();
283:        sleep(Duration::from_millis(100)).await;
284:        let elapsed = start.elapsed();
285:
286:        assert!(elapsed >= Duration::from_millis(90));
287:    }
288:
289:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
290:    #[tokio::test]
291:    async fn test_interval() {
292:        let mut interval = Interval::new(Duration::from_millis(50));
293:
294:        // First tick should complete immediately
295:        let start = std::time::Instant::now();
296:        interval.tick().await;
297:        let first_tick = start.elapsed();
298:        assert!(first_tick < Duration::from_millis(10));
299:
300:        // Second tick should wait
301:        let start = std::time::Instant::now();
302:        interval.tick().await;
303:        let second_tick = start.elapsed();
304:        assert!(second_tick >= Duration::from_millis(40));
305:    }
306:
307:    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
308:    #[wasm_bindgen_test::wasm_bindgen_test]
309:    async fn test_interval_browser() {
310:        let mut interval = Interval::new(Duration::from_millis(50));
311:
312:        // In browser implementation, first tick also waits
313:        interval.tick().await;
314:
315:        let start = instant::Instant::now();
316:        interval.tick().await;
317:        let elapsed = start.elapsed();
318:
319:        assert!(elapsed >= Duration::from_millis(40));
320:    }
321:
322:    #[test]
323:    fn test_system_time() {
324:        let now = SystemTime::now();
325:        let duration_since_epoch = now.duration_since(UNIX_EPOCH);
326:
327:        // Should be successful (we're past the epoch)
328:        assert!(duration_since_epoch.is_ok());
329:
330:        // Should be a reasonable time (after year 2020)
331:        let duration = duration_since_epoch.unwrap();
332:        assert!(duration.as_secs() > 1_600_000_000);
333:    }
334:
335:    #[test]
336:    fn test_system_time_ordering() {
337:        let t1 = SystemTime::now();
338:        std::thread::sleep(Duration::from_millis(10));
339:        let t2 = SystemTime::now();
340:
341:        // t2 should be after t1
342:        assert!(t2 > t1);
343:    }
344:
345:    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
346:    #[tokio::test]
347:    async fn test_multiple_intervals() {
348:        use std::sync::Arc;
349:        use std::sync::atomic::{AtomicUsize, Ordering};
350:
351:        let counter = Arc::new(AtomicUsize::new(0));
352:        let counter_clone = counter.clone();
353:
354:        tokio::spawn(async move {
355:            let mut interval = Interval::new(Duration::from_millis(25));
356:            for _ in 0..3 {
357:                interval.tick().await;
358:                counter_clone.fetch_add(1, Ordering::SeqCst);
359:            }
360:        });
361:
362:        tokio::time::sleep(Duration::from_millis(150)).await;
363:
364:        assert_eq!(counter.load(Ordering::SeqCst), 3);
365:    }
366:}
```


--- a/Cargo.toml
+++ b/Cargo.toml
``` toml
 1:[package]
 2:name = "aloeplatform"
 3:version = "0.1.0"
 4:edition = "2024"
 5:
 6:[lib]
 7:name = "aloeplatform"
 8:path = "src/lib.rs"
 9:
10:[[bin]]
11:name = "aloeplatform-example"
12:path = "src/main.rs"
13:
14:[[test]]
15:name = "integration-tests"
16:path = "tests/integration.rs"
17:
18:[[test]]
19:name = "io-integration-tests"
20:path = "tests/io_integration.rs"
21:
22:[dependencies]
23:log = "0.4"
24:futures = "0.3"
25:base64 = "0.22"
26:
27:
28:# # Native only
29:[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
30:tokio = { version = "1", features = ["full"] }
31:
32:# # --- WASM Common (Both CLI and Browser) ---
33:[target.'cfg(target_arch = "wasm32")'.dependencies]
34:tokio = { version = "1", features = ["sync", "macros", "io-util", "rt", "time"] }
35:futures = "0.3"
36:
37:# # --- Not Browser ----
38:[target.'cfg(not(all(target_arch = "wasm32", target_os = "unknown")))'.dependencies]
39:env_logger = "0.11" # env_logger picks up env vars so long as native, or wasmtime
40:
41:# # WASI P2 only
42:[target.'cfg(all(target_arch = "wasm32", target_env = "p2"))'.dependencies]
43:tokio = { version = "1", features = ["sync", "macros", "io-util", "rt", "time"] }
44:wasi = "0.14"
45:
46:# # Browser only
47:[target.'cfg(all(target_arch = "wasm32", target_os = "unknown"))'.dependencies]
48:wasm-bindgen = "0.2"
49:wasm-bindgen-futures = "0.4"
50:console_error_panic_hook = "0.1"
51:console_log = "1"
52:wasm-logger = "0.2.0"
53:gloo-timers = { version = "0.3", features = ["futures"] }
54:web-time = "1.1"
55:instant = { version = "0.1", features = ["wasm-bindgen"] }
56:web-sys = { version = "0.3", features = ["Window", "Storage"]}
57:
58:[dev-dependencies]
59:# For testing async code
60:tokio-test = "0.4"
61:
62:# Browser test dependencies
63:[target.'cfg(all(target_arch = "wasm32", target_os = "unknown"))'.dev-dependencies]
64:wasm-bindgen-test = "0.3"
```
