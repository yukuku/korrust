import java.util.ArrayList;
import java.util.Arrays;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Random;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import fixtures.io.IoTrace;
import korrust.probes.io.ByteWorkbench;
import korrust.probes.io.ArrayEdgesKt;
import korrust.probes.io.HiddenBuffers;
import korrust.probes.io.LazyScratch;
import korrust.probes.io.StaticReceiver;

public final class BytesCheck {
    private static String failure(Runnable action) {
        try { action.run(); return "success"; }
        catch (RuntimeException error) { return error.getClass().getName(); }
    }
    private static void ordered(byte[] bytes, char[] chars, int position, byte value, boolean failIndex, boolean failValue, boolean characters) {
        IoTrace expected = new IoTrace(bytes == null ? null : bytes.clone(), chars == null ? null : chars.clone(), position, value, failIndex, failValue);
        IoTrace actual = new IoTrace(bytes == null ? null : bytes.clone(), chars == null ? null : chars.clone(), position, value, failIndex, failValue);
        String wanted = failure(() -> {
            if (characters) expected.charArray()[expected.index()] = expected.charValue();
            else expected.array()[expected.index()] = expected.value();
        });
        InterfaceAssertions.record(failure(() -> {
            if (characters) ByteWorkbench.orderedChars(actual);
            else ByteWorkbench.ordered(actual);
        }), wanted);
        InterfaceAssertions.record(actual.trace, expected.trace);
        InterfaceAssertions.record(Arrays.toString(actual.bytes), Arrays.toString(expected.bytes));
        InterfaceAssertions.record(Arrays.toString(actual.chars), Arrays.toString(expected.chars));
    }
    public static void main(String[] args) throws Exception {
        InterfaceAssertions.record(InterfaceAssertions.failure(StaticReceiver::run), "java.lang.IllegalArgumentException:receiver-evaluated");
        InterfaceAssertions.record(ByteWorkbench.INSTANCE.instanceCode(), 23);
        for (int value : new int[] {-1, 0, 1, 2}) InterfaceAssertions.record(ByteWorkbench.whenValue(value), value == 1 ? 2 : 3);
        char[] created = ArrayEdgesKt.newChars(3);
        InterfaceAssertions.record(Arrays.toString(created), Arrays.toString(new char[3]));
        ArrayEdgesKt.writeChar(created, 1, '\ud800');
        InterfaceAssertions.record((int)created[1], 0xd800);
        InterfaceAssertions.record(failure(() -> ArrayEdgesKt.newChars(-1)), "java.lang.NegativeArraySizeException");
        byte[] all = new byte[256];
        for (int i = 0; i < all.length; i++) all[i] = (byte)i;
        for (int i = 0; i < all.length; i++) {
            InterfaceAssertions.record(ByteWorkbench.pick(all[i]), (int)all[i]);
            InterfaceAssertions.record(ByteWorkbench.pick(i), i + 11);
            InterfaceAssertions.record(ByteWorkbench.pick(all, i), all[i]);
            InterfaceAssertions.record(ByteWorkbench.narrow(i), (byte)i);
            InterfaceAssertions.record(ByteWorkbench.classify(all[i]), i < 32 ? 3 : i <= 126 ? 5 : 9);
            byte[] changed = new byte[3];
            InterfaceAssertions.record(ByteWorkbench.put(changed, 1, all[i]), all[i]);
            InterfaceAssertions.record(changed[1], all[i]);
        }
        for (int value : new int[] {Integer.MIN_VALUE, -65537, -256, -129, -128, -1, 0, 127, 128, 255, 256, 65535, 65536, Integer.MAX_VALUE}) {
            InterfaceAssertions.record(ByteWorkbench.narrow(value), (byte)value);
            InterfaceAssertions.record(ByteWorkbench.pick(value), value + 11);
        }
        StringBuilder expectedChars = new StringBuilder();
        for (int i = 0; i < 256; i++) expectedChars.append((char)i);
        InterfaceAssertions.record(ByteWorkbench.chars(all), expectedChars.toString());
        for (int unit = 0; unit <= 65535; unit++) {
            char[] chars = new char[3];
            InterfaceAssertions.record(ByteWorkbench.fill(chars, 1, (char)unit), 2);
            InterfaceAssertions.record((int)chars[1], unit);
        }
        Random random = new Random(381742);
        for (int sample = 0; sample < 128; sample++) {
            byte[] input = new byte[random.nextInt(257)];
            random.nextBytes(input);
            int expected = 0;
            for (byte element : input) {
                int code = element & 255;
                if (code == 0) continue;
                if (code == 255) break;
                expected += code;
            }
            InterfaceAssertions.record(ByteWorkbench.fold(input), expected);
        }
        for (boolean flag : new boolean[] {false, true}) {
            for (int left : new int[] {Integer.MIN_VALUE, -1, 0, 1, Integer.MAX_VALUE}) {
                for (int right : new int[] {Integer.MIN_VALUE, -1, 0, 1, Integer.MAX_VALUE}) InterfaceAssertions.record(ByteWorkbench.assigned(flag, left, right), flag ? left : right);
            }
        }
        for (int length : new int[] {0, 1, 4, 8}) {
            char[] source = {'A', '\u0000', '\ud800', '\uffff'};
            char[] result = ByteWorkbench.resized(source, length);
            InterfaceAssertions.record(Arrays.toString(result), Arrays.toString(Arrays.copyOf(source, length)));
            InterfaceAssertions.record(result == source, false);
        }
        for (int index : new int[] {Integer.MIN_VALUE, -1, 3, Integer.MAX_VALUE}) {
            InterfaceAssertions.record(failure(() -> ByteWorkbench.pick(new byte[3], index)), "java.lang.ArrayIndexOutOfBoundsException");
            InterfaceAssertions.record(failure(() -> ByteWorkbench.put(new byte[3], index, (byte)1)), "java.lang.ArrayIndexOutOfBoundsException");
            InterfaceAssertions.record(failure(() -> ByteWorkbench.fill(new char[3], index, 'x')), "java.lang.ArrayIndexOutOfBoundsException");
        }
        for (boolean characters : new boolean[] {false, true}) {
            for (boolean missing : new boolean[] {false, true}) {
                for (boolean failIndex : new boolean[] {false, true}) {
                    for (boolean failValue : new boolean[] {false, true}) {
                        for (int index : new int[] {-1, 0, 1, Integer.MAX_VALUE}) ordered(missing ? null : new byte[1], missing ? null : new char[1], index, (byte)-1, failIndex, failValue, characters);
                    }
                }
            }
        }
        for (boolean missing : new boolean[] {false, true}) {
            for (boolean failIndex : new boolean[] {false, true}) {
                for (boolean failValue : new boolean[] {false, true}) {
                    for (int position : new int[] {-1, 0, 1}) {
                        IoTrace expected = new IoTrace(null, missing ? null : new char[] {'Q'}, position, (byte)0, failIndex, failValue);
                        IoTrace actual = new IoTrace(null, missing ? null : new char[] {'Q'}, position, (byte)0, failIndex, failValue);
                        String[] wanted = new String[1], observed = new String[1];
                        String failure = failure(() -> wanted[0] = new String(java.util.Objects.requireNonNull(expected.charArray()), expected.index(), expected.length()));
                        InterfaceAssertions.record(failure(() -> observed[0] = ByteWorkbench.constructed(actual)), failure);
                        InterfaceAssertions.record(actual.trace, expected.trace);
                        if (failure.equals("success")) InterfaceAssertions.record(observed[0], wanted[0]);
                    }
                }
            }
        }
        for (int bound : new int[] {-3, 0, 1, 2, 8}) {
            IoTrace loop = new IoTrace(null, null, bound, (byte)0, false, false);
            int sum = 0;
            for (int i = 0; i < bound; i++) sum += i;
            InterfaceAssertions.record(ByteWorkbench.endpoints(loop), sum);
            InterfaceAssertions.record(loop.trace, "E");
            IoTrace range = new IoTrace(null, null, bound, (byte)0, false, false);
            InterfaceAssertions.record(ByteWorkbench.ranged(range), bound >= 0);
            InterfaceAssertions.record(range.trace, "LHS");
            IoTrace selection = new IoTrace(null, null, bound, (byte)0, false, false);
            InterfaceAssertions.record(ByteWorkbench.selected(selection), bound >= 0 && bound <= 5 ? 7 : 9);
            InterfaceAssertions.record(selection.trace, "S");
        }
        InterfaceAssertions.record(ByteWorkbench.scratchBytes().length, 5);
        InterfaceAssertions.record(ByteWorkbench.scratchChars().length, 7);
        InterfaceAssertions.record(ByteWorkbench.scratchBytes() == ByteWorkbench.scratchBytes(), true);
        InterfaceAssertions.record(ByteWorkbench.scratchChars() == ByteWorkbench.scratchChars(), true);
        byte[] replacement = {4, 2};
        ByteWorkbench.setBytes(replacement);
        InterfaceAssertions.record(ByteWorkbench.scratchBytes() == replacement, true);
        InterfaceAssertions.record(HiddenBuffers.size(), 3);
        InterfaceAssertions.record(LazyScratch.constant(), 17);
        for (int repetition = 0; repetition < 2; repetition++) {
            InterfaceAssertions.record(failure(LazyScratch::touchBytes), "java.lang.NegativeArraySizeException");
            InterfaceAssertions.record(failure(LazyScratch::touchChars), "java.lang.NegativeArraySizeException");
            InterfaceAssertions.record(LazyScratch.constant(), 17);
        }
        ExecutorService executor = Executors.newFixedThreadPool(4);
        CyclicBarrier barrier = new CyclicBarrier(4);
        List<Future<Object[]>> futures = new ArrayList<>();
        try {
            for (int task = 0; task < 4; task++) futures.add(executor.submit(() -> {
                barrier.await();
                byte[] bytes = ByteWorkbench.scratchBytes();
                char[] chars = ByteWorkbench.scratchChars();
                if (bytes.length != 5 || chars.length != 7 || bytes != ByteWorkbench.scratchBytes() || chars != ByteWorkbench.scratchChars()) throw new AssertionError("Thread-local initialization mismatch");
                return new Object[] {bytes, chars};
            }));
            IdentityHashMap<Object, Boolean> identities = new IdentityHashMap<>();
            for (Future<Object[]> future : futures) for (Object array : future.get()) InterfaceAssertions.record(identities.put(array, true), null);
            InterfaceAssertions.record(identities.size(), 8);
        } finally { executor.shutdownNow(); }
        InterfaceAssertions.finish();
    }
}
