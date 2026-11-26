import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Supplier;
import java.util.List;
import java.util.Objects;
import callbackfunctions.ValuesKt;
import receivercallbacks.Activity;
import receiverlibrary.LibraryKt;
import receiverlibrary.Pending;
public final class Check {
    static void suspended(Supplier<Object> action, int count, Object expected) throws Exception {
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
        Object actual = action.get(); resumer.join(6000);
        if (resumer.isAlive() || failure.get() != null || !Objects.equals(actual, expected)) throw new AssertionError("result " + actual, failure.get());
    }
    public static void main(String[] args) throws Exception {
        Activity.INSTANCE.setCreations(0);
        if (!ValuesKt.imported().equals(List.of(5, 6, 7)) || Activity.INSTANCE.getCreations() != 1) throw new AssertionError();
        if (!ValuesKt.shadowed().equals(List.of(2, 4, 6)) || Activity.INSTANCE.getCreations() != 2) throw new AssertionError();
        suspended(ValuesKt::asyncs, 3, List.of(9, 9, 9));
        suspended(ValuesKt::explicit, 3, List.of(9, 9, 9));
        suspended(ValuesKt::dispatched, 3, List.of(9, 9, 9));
        suspended(ValuesKt::size, 1, 3);
        suspended(ValuesKt::empty, 1, false);
        suspended(ValuesKt::arraySize, 1, 2);
        suspended(ValuesKt::length, 1, 3);
        suspended(() -> {
            try { ValuesKt.failedLength(); throw new AssertionError("missing null failure"); }
            catch (NullPointerException expected) { return 7; }
        }, 1, 7);
        System.out.println("OK");
    }
}
