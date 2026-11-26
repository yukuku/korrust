import java.util.concurrent.atomic.AtomicReference;
import java.util.function.IntSupplier;
import receiverfunctions.ValuesKt;
import receiverlibrary.LibraryKt;
import receiverlibrary.Pending;
public final class Check {
    static void suspended(IntSupplier action, int expected) throws Exception {
        Thread caller = Thread.currentThread();
        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread resumer = new Thread(() -> {
            try {
                Pending pending = LibraryKt.takePending();
                long deadline = System.nanoTime() + 5_000_000_000L;
                while (pending.getThread().getState() != Thread.State.WAITING && pending.getThread().getState() != Thread.State.TIMED_WAITING) {
                    if (System.nanoTime() >= deadline) throw new AssertionError("coroutine did not suspend");
                    Thread.sleep(1);
                }
                LibraryKt.resumePending(pending);
            } catch (Throwable error) { failure.set(error); caller.interrupt(); }
        });
        resumer.setDaemon(true);
        resumer.start();
        int actual = action.getAsInt();
        resumer.join(6000);
        if (resumer.isAlive() || failure.get() != null || actual != expected) throw new AssertionError("suspend result " + actual, failure.get());
    }
    public static void main(String[] args) throws Exception {
        suspended(ValuesKt::compute, 14);
        suspended(() -> ValuesKt.captured(4), 13);
        suspended(ValuesKt::invokeValue, 9);
        suspended(ValuesKt::ordinary, 9);
        if (ValuesKt.invokeReceiver((scope, continuation) -> { if (scope == null) throw new AssertionError(); return 11; }) != 11) throw new AssertionError();
        System.out.println("OK");
    }
}
