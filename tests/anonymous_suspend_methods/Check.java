import java.util.Objects;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.Supplier;
import anonymousfunctions.ValuesKt;
import anonymouslibrary.Service;
import kotlin.coroutines.Continuation;
import kotlin.coroutines.CoroutineContext;
import kotlin.coroutines.EmptyCoroutineContext;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
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
        if (!ValuesKt.narrow().equals("ok") || !ValuesKt.declared().name().equals("declared")) throw new AssertionError();
        Service value = ValuesKt.make(4);
        suspended(() -> ValuesKt.first(value, 2), 1, 16);
        if (value.ready() != 16) throw new AssertionError();
        suspended(() -> ValuesKt.second(value, 3), 2, 37);
        suspended(() -> ValuesKt.text(value, "x"), 1, "x9");
        if (ValuesKt.text(value, "null") != null) throw new AssertionError();
        suspended(() -> ValuesKt.finish(value, 1), 1, 26);
        suspended(() -> ValuesKt.echo(ValuesKt.generic(), "g"), 1, "g9");
        suspended(() -> ValuesKt.get(ValuesKt.base(5), 2), 1, 16);
        suspended(() -> ValuesKt.echoInt(ValuesKt.genericInt(), 3), 1, 12);
        anonymouslibrary.SuspendSam sam = ValuesKt.sam(4);
        if (sam instanceof kotlin.coroutines.jvm.internal.SuspendFunction) throw new AssertionError();
        suspended(() -> ValuesKt.callSam(sam, 2), 1, 15);
        suspended(() -> ValuesKt.callGenericSam(ValuesKt.genericSam(), "s"), 1, "s9");
        suspended(() -> ValuesKt.callUnitSam(ValuesKt.unitSam()), 1, 7);
        suspended(() -> ValuesKt.callWide(ValuesKt.wide()), 1, 18L);
        Service direct = ValuesKt.make(3);
        AtomicReference<Object> resumed = new AtomicReference<>();
        Continuation<Integer> continuation = new Continuation<>() {
            public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
            public void resumeWith(Object result) { resumed.set(result); }
        };
        Object marker = direct.first(2, continuation);
        if (marker != IntrinsicsKt.getCOROUTINE_SUSPENDED() || resumed.get() != null) throw new AssertionError();
        LibraryKt.resumePending(LibraryKt.takePending());
        if (!Objects.equals(resumed.get(), 15) || direct.ready() != 15) throw new AssertionError();
        System.out.println("OK");
    }
}
