public final class Calls {
 private static void eq(Object a,Object b) { if (!java.util.Objects.equals(a,b)) throw new AssertionError(a+" != "+b); }
 static void suspended(java.util.function.IntSupplier action, int expected) throws Exception {
  Thread caller = Thread.currentThread();
  java.util.concurrent.atomic.AtomicReference<Throwable> failure = new java.util.concurrent.atomic.AtomicReference<>();
  Thread resumer = new Thread(() -> {
   try {
    targets.Pending pending = targets.LibraryKt.takePending();
    long deadline = System.nanoTime() + 5_000_000_000L;
    while (pending.getThread().getState()!=Thread.State.WAITING && pending.getThread().getState()!=Thread.State.TIMED_WAITING) {
     if (System.nanoTime()>deadline) throw new AssertionError("did not suspend");
     Thread.sleep(1);
    }
    targets.LibraryKt.resumePending(pending);
   } catch(Throwable error) { failure.set(error); caller.interrupt(); }
  });
  resumer.setDaemon(true); resumer.start();
  int actual=action.getAsInt(); resumer.join(6000);
  if (resumer.isAlive() || failure.get()!=null || actual!=expected) throw new AssertionError("suspend result "+actual, failure.get());
 }
 public static void main(String[] args) throws Exception {
  eq(EffectsKt.local(true),"early");eq(EffectsKt.local(false),"late");
  eq(EffectsKt.outer(true),7);eq(EffectsKt.outer(false),9);eq(EffectsKt.middle(),13);eq(EffectsKt.shadow(),5);
  Probe.trace="";eq(EffectsKt.unwind(),7);eq(Probe.trace,"BC");
  Probe.trace="";eq(EffectsKt.partial(),4);eq(Probe.trace,"CAC");
  suspended(() -> EffectsKt.resumed(true),11);
  suspended(() -> EffectsKt.resumed(false),13);
  suspended(EffectsKt::resumedUnit,1); eq(Probe.trace,"U");
  System.out.println("OK");
 }
}
