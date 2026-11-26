import java.util.concurrent.atomic.AtomicReference;
import java.util.function.IntSupplier;
import operandfunctions.*;
import receiverlibrary.LibraryKt;
import receiverlibrary.Pending;
public final class OperandCheck {
    static void suspended(IntSupplier action, int count, int expected, int trace) throws Exception {
        Events.INSTANCE.setTrace(0);
        Thread caller = Thread.currentThread();
        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread resumer = new Thread(() -> {
            try {
                for (int index = 0; index < count; index++) {
                    Pending pending = LibraryKt.takePending();
                    long deadline = System.nanoTime() + 5_000_000_000L;
                    while (pending.getThread().getState() != Thread.State.WAITING && pending.getThread().getState() != Thread.State.TIMED_WAITING) {
                        if (System.nanoTime() >= deadline) throw new AssertionError("coroutine did not suspend");
                        Thread.sleep(1);
                    }
                    LibraryKt.resumePending(pending);
                }
            } catch (Throwable error) { failure.set(error); caller.interrupt(); }
        });
        resumer.setDaemon(true); resumer.start();
        int actual = action.getAsInt(); resumer.join(6000);
        if (resumer.isAlive() || failure.get() != null || actual != expected || Events.INSTANCE.getTrace() != trace) throw new AssertionError("result " + actual + " trace " + Events.INSTANCE.getTrace(), failure.get());
    }
    public static void main(String[] args) throws Exception {
        suspended(ValuesKt::ordered, 1, 17, 123465);
        suspended(ValuesKt::nested, 2, 18, 3434);
        suspended(ValuesKt::variable, 1, 11, 12345);
        suspended(ValuesKt::direct, 1, 11, 2345);
        Events.INSTANCE.setTrace(0);
        try { ValuesKt.beforeFailure(); throw new AssertionError("missing failure"); }
        catch (IllegalStateException expected) { if (Events.INSTANCE.getTrace() != 2) throw new AssertionError(); }
        suspended(() -> {
            try { ValuesKt.afterFailure(); throw new AssertionError("missing failure"); }
            catch (IllegalStateException expected) { return Events.INSTANCE.getTrace(); }
        }, 1, 342, 342);
        System.out.println("OK");
    }
}
