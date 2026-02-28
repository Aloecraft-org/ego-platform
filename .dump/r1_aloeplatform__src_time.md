
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
