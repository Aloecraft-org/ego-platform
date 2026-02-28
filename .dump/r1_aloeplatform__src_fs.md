
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
