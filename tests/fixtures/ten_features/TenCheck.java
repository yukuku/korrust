import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;
import ten.LongOpsKt;
import ten.Frames;
import ten.PublicFields;
import ten.ScalarRecord;
import ten.TenOpsKt;
import ten.ThirtyTwo;
import ten.trace.Trace;
import yuku.alkitab.base.audio.AudioLogEntry;
import yuku.alkitab.base.dialog.VersesDialogNormalVerses;
import yuku.alkitab.base.dialog.XrefDialogVerses;
import yuku.alkitab.model.SingleChapterVerses;

public final class TenCheck {
    private interface Action { void run() throws Exception; }
    private static int checks;
    private static final Trace TRACE = new Trace();

    private static void eq(Object actual, Object expected) {
        checks++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("Expected " + expected + ", found " + actual);
    }

    private static void fails(Class<? extends Throwable> expected, Action action) throws Exception {
        checks++;
        try { action.run(); } catch (Throwable error) {
            if (error instanceof InvocationTargetException) error = error.getCause();
            if (expected.isInstance(error)) return;
            throw new AssertionError("Expected " + expected.getName(), error);
        }
        throw new AssertionError("Expected " + expected.getName());
    }

    private static void observe(String id, Action action) throws Exception {
        int start = checks;
        action.run();
        System.out.println("{\"id\":\"" + id + "\",\"outcome\":\"pass\",\"checks\":" + (checks - start) + "}");
    }

    private static void longs() throws Exception {
        long[] values = {Long.MIN_VALUE, Long.MIN_VALUE + 1, -4294967296L, -2147483649L, -1L, 0L, 1L, 2147483648L, 4294967296L, Long.MAX_VALUE};
        for (long a : values) {
            eq(LongOpsKt.longNegate(a), -a);
            eq(LongOpsKt.longText(a), Long.toString(a));
            for (long b : values) {
                eq(LongOpsKt.longAdd(a, b), a + b);
                eq(LongOpsKt.longSubtract(a, b), a - b);
                eq(LongOpsKt.longMultiply(a, b), a * b);
                eq(LongOpsKt.longAnd(a, b), a & b);
                eq(LongOpsKt.longOr(a, b), a | b);
                eq(LongOpsKt.longXor(a, b), a ^ b);
                eq(LongOpsKt.longEqual(a, b), a == b);
                eq(LongOpsKt.longNotEqual(a, b), a != b);
                eq(LongOpsKt.longLess(a, b), a < b);
                eq(LongOpsKt.longLessEqual(a, b), a <= b);
                eq(LongOpsKt.longGreater(a, b), a > b);
                eq(LongOpsKt.longGreaterEqual(a, b), a >= b);
                if (b == 0) {
                    fails(ArithmeticException.class, () -> LongOpsKt.longDivide(a, b));
                    fails(ArithmeticException.class, () -> LongOpsKt.longRemainder(a, b));
                } else {
                    eq(LongOpsKt.longDivide(a, b), a / b);
                    eq(LongOpsKt.longRemainder(a, b), a % b);
                }
                for (boolean flag : new boolean[] {false, true}) {
                    for (int delta : new int[] {-1, 0, 7}) {
                        long result = flag ? a + (b + 3L) : a - (b + 3L);
                        eq(LongOpsKt.longSlots(a, flag, b, delta), delta < 0 ? -result : result);
                    }
                }
            }
            for (int shift : new int[] {Integer.MIN_VALUE, -65, -64, -1, 0, 1, 31, 32, 63, 64, 65, Integer.MAX_VALUE}) {
                eq(LongOpsKt.longLeft(a, shift), a << shift);
                eq(LongOpsKt.longRight(a, shift), a >> shift);
                eq(LongOpsKt.longUnsignedRight(a, shift), a >>> shift);
            }
        }
        eq(LongOpsKt.longMinimum(), Long.MIN_VALUE);
        eq(LongOpsKt.longMaximum(), Long.MAX_VALUE);
        eq(LongOpsKt.longConstants(), -1L);
        eq(LongOpsKt.longHex(), Long.MAX_VALUE);
        eq(LongOpsKt.longBinary(), 165L);
        eq(LongOpsKt.longExpectedLiteral(), 17L);
    }

    private static void publicProperties() throws Exception {
        List<String> texts = new ArrayList<>(Arrays.asList("first", null));
        PublicFields value = new PublicFields(-7, Long.MIN_VALUE, "\u0000\ud800", true, "title", null, texts, 42);
        eq(value.getCount(), -7);
        eq(value.getTimestamp(), Long.MIN_VALUE);
        eq(value.getLabel(), "\u0000\ud800");
        eq(value.isReady(), true);
        eq(value.isTitle(), "title");
        eq(value.getOptional(), null);
        eq(value.getTexts() == texts, true);
        texts.set(0, "changed");
        eq(value.getTexts().get(0), "changed");
        for (String name : new String[] {"count", "timestamp", "label", "isReady", "isTitle", "optional", "texts", "hidden"}) {
            eq(PublicFields.class.getDeclaredField(name).getModifiers(), Modifier.PRIVATE | Modifier.FINAL);
        }
        eq(Arrays.stream(PublicFields.class.getDeclaredMethods()).anyMatch(method -> method.getName().startsWith("set")), false);
        fails(NoSuchMethodException.class, () -> PublicFields.class.getDeclaredMethod("getHidden"));
        fails(NullPointerException.class, () -> new PublicFields(0, 0L, null, false, "title", null, texts, 0));
        fails(NullPointerException.class, () -> new PublicFields(0, 0L, "label", false, null, null, texts, 0));
        fails(NullPointerException.class, () -> new PublicFields(0, 0L, "label", false, "title", null, null, 0));
    }

    private static void data() throws Exception {
        Method copy = ScalarRecord.class.getDeclaredMethod("copy$default", ScalarRecord.class, int.class, byte.class, char.class, boolean.class, long.class, String.class, int.class, Object.class);
        for (long timestamp : new long[] {Long.MIN_VALUE, -1L, 0L, Long.MAX_VALUE}) {
            ScalarRecord value = new ScalarRecord(-17, (byte) -128, '\ud800', true, timestamp, "\u0000\udfff");
            eq(value.component1(), -17);
            eq(value.component2(), (byte) -128);
            eq(value.component3(), '\ud800');
            eq(value.component4(), true);
            eq(value.component5(), timestamp);
            eq(value.component6(), "\u0000\udfff");
            eq(value.equals(value), true);
            eq(value.equals(null), false);
            eq(value.equals("wrong class"), false);
            eq(value.equals(new ScalarRecord(-17, (byte) -128, '\ud800', true, timestamp, "\u0000\udfff")), true);
            int hash = -17;
            hash = hash * 31 - 128;
            hash = hash * 31 + '\ud800';
            hash = hash * 31 + Boolean.hashCode(true);
            hash = hash * 31 + Long.hashCode(timestamp);
            hash = hash * 31 + "\u0000\udfff".hashCode();
            eq(value.hashCode(), hash);
            eq(value.toString(), "ScalarRecord(count=-17, tiny=-128, character=\ud800, isReady=true, timestamp=" + timestamp + ", message=\u0000\udfff)");
            for (int mask = 0; mask < 64; mask++) {
                ScalarRecord expected = new ScalarRecord((mask & 1) != 0 ? -17 : 9, (mask & 2) != 0 ? (byte) -128 : (byte) 3, (mask & 4) != 0 ? '\ud800' : 'z', (mask & 8) != 0, (mask & 16) != 0 ? timestamp : 7L, (mask & 32) != 0 ? "\u0000\udfff" : "new");
                ScalarRecord actual = (ScalarRecord) copy.invoke(null, value, 9, (byte) 3, 'z', false, 7L, "new", mask, null);
                eq(actual, expected);
                eq(actual.hashCode(), expected.hashCode());
                eq(actual == value, false);
            }
            eq(value.copy(1, (byte) 2, '3', false, 4L, "5"), new ScalarRecord(1, (byte) 2, '3', false, 4L, "5"));
            fails(NullPointerException.class, () -> value.copy(0, (byte) 0, 'x', false, 0L, null));
        }
        fails(NullPointerException.class, () -> new ScalarRecord(0, (byte) 0, 'x', false, 0L, null));
        AudioLogEntry entry = new AudioLogEntry(Long.MIN_VALUE, "message");
        eq(entry.component1(), Long.MIN_VALUE);
        eq(entry.component2(), "message");
        eq(entry.copy(Long.MAX_VALUE, "changed"), new AudioLogEntry(Long.MAX_VALUE, "changed"));
        eq(entry.toString(), "AudioLogEntry(timestampMs=" + Long.MIN_VALUE + ", message=message)");
        Method audioCopy = AudioLogEntry.class.getDeclaredMethod("copy$default", AudioLogEntry.class, long.class, String.class, int.class, Object.class);
        for (int mask = 0; mask < 4; mask++) {
            eq(audioCopy.invoke(null, entry, 7L, "new", mask, null), new AudioLogEntry((mask & 1) != 0 ? Long.MIN_VALUE : 7L, (mask & 2) != 0 ? "message" : "new"));
        }
        Class<?>[] parameters = new Class<?>[32];
        Arrays.fill(parameters, int.class);
        Constructor<ThirtyTwo> constructor = ThirtyTwo.class.getDeclaredConstructor(parameters);
        Object[] arguments = new Object[32];
        for (int index = 0; index < 32; index++) arguments[index] = index + 1;
        ThirtyTwo thirtyTwo = constructor.newInstance(arguments);
        Class<?>[] copyParameters = new Class<?>[35];
        copyParameters[0] = ThirtyTwo.class;
        Arrays.fill(copyParameters, 1, 34, int.class);
        copyParameters[34] = Object.class;
        Method largeCopy = ThirtyTwo.class.getDeclaredMethod("copy$default", copyParameters);
        for (int mask : new int[] {0, -1, Integer.MIN_VALUE, 0x55555555, 0xaaaaaaaa}) {
            Object[] values = new Object[35];
            values[0] = thirtyTwo;
            Arrays.fill(values, 1, 33, -7);
            values[33] = mask;
            values[34] = null;
            ThirtyTwo actual = (ThirtyTwo) largeCopy.invoke(null, values);
            for (int index = 0; index < 32; index++) {
                eq(ThirtyTwo.class.getDeclaredMethod("component" + (index + 1)).invoke(actual), (mask & (1 << index)) != 0 ? index + 1 : -7);
            }
        }
    }

    private static void nullable() throws Exception {
        eq(TenOpsKt.nullableLiteral(), null);
        for (String a : new String[] {null, "", "value", "\u0000\ud800"}) {
            eq(TenOpsKt.nullableIdentity(a), a);
            eq(TenOpsKt.nullableEqual(a), a == null);
            eq(TenOpsKt.nullableNotEqual(a), a != null);
            for (boolean condition : new boolean[] {false, true}) {
                Trace.firstCondition = condition;
                Trace.trace = "";
                eq(TenOpsKt.nullConditionEqual(TRACE, a), a == null);
                eq(Trace.trace, "aR");
            }
        }
    }

    private static void safe() {
        for (String value : new String[] {null, "", "xvalue", "value"}) {
            Trace.trace = "";
            eq(TenOpsKt.safeRemove(TRACE, value), value == null ? null : value.startsWith("x") ? value.substring(1) : value);
            eq(Trace.trace, value == null ? "R" : "RA");
            Trace.trace = "";
            eq(TenOpsKt.safeChain(TRACE, value), value == null ? null : value.startsWith("x") ? value.substring(1) : value);
            eq(Trace.trace, "R");
        }
    }

    private static void elvis() {
        for (boolean condition : new boolean[] {false, true}) {
            Trace.firstCondition = condition;
            Trace.trace = "";
            eq(TenOpsKt.nullConditionElvis(TRACE), "fallback");
            eq(Trace.trace, "aF");
        }
        for (String value : new String[] {null, "", "xvalue", "value"}) {
            Trace.trace = "";
            eq(TenOpsKt.elvis(TRACE, value), value == null ? "fallback" : value);
            eq(Trace.trace, value == null ? "RF" : "R");
            Trace.trace = "";
            eq(TenOpsKt.safeElvis(TRACE, value), value == null ? "fallback" : value.startsWith("x") ? value.substring(1) : value);
            eq(Trace.trace, value == null ? "RF" : "RA");
        }
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static void lists() throws Exception {
        List<String> verses = new ArrayList<>(Arrays.asList("first", "\u0000\ud800", null));
        List<String> numbers = new ArrayList<>(Arrays.asList("1", "2", "3"));
        List<SingleChapterVerses> chapters = Arrays.asList(new VersesDialogNormalVerses(verses, numbers), new XrefDialogVerses("missing", verses, numbers));
        eq(TenOpsKt.listSize(verses), 3);
        eq(TenOpsKt.listGet(verses, 1), "\u0000\ud800");
        eq(TenOpsKt.nullableListGet(verses, 2), null);
        Trace.trace = "";
        eq(TenOpsKt.listFallback(TRACE, verses, 2), "fallback");
        eq(Trace.trace, "F");
        Trace.trace = "";
        eq(TenOpsKt.listFallback(TRACE, verses, 0), "first");
        eq(Trace.trace, "");
        for (SingleChapterVerses chapter : chapters) {
            eq(chapter.getVerseCount(), 3);
            eq(chapter.getVerse(1), "\u0000\ud800");
            eq(chapter.getVerseNumberText(2), "3");
            for (int index : new int[] {Integer.MIN_VALUE, -1, 3, Integer.MAX_VALUE}) {
                fails(IndexOutOfBoundsException.class, () -> chapter.getVerse(index));
                fails(IndexOutOfBoundsException.class, () -> chapter.getVerseNumberText(index));
            }
        }
        eq(chapters.get(0).getVerse(2), null);
        eq(chapters.get(1).getVerse(2), "missing");
        verses.set(0, "changed");
        numbers.set(0, "changed number");
        for (SingleChapterVerses chapter : chapters) {
            eq(chapter.getVerse(0), "changed");
            eq(chapter.getVerseNumberText(0), "changed number");
        }
        List raw = new ArrayList();
        raw.add(17);
        fails(ClassCastException.class, () -> TenOpsKt.listGet(raw, 0));
        fails(ClassCastException.class, () -> TenOpsKt.nullableListGet(raw, 0));
        fails(ClassCastException.class, () -> new VersesDialogNormalVerses(raw, numbers).getVerse(0));
        fails(ClassCastException.class, () -> new XrefDialogVerses("missing", raw, numbers).getVerse(0));
        fails(NullPointerException.class, () -> TenOpsKt.listSize(null));
        fails(NullPointerException.class, () -> new VersesDialogNormalVerses(null, numbers));
        fails(NullPointerException.class, () -> new XrefDialogVerses(null, verses, numbers));
    }

    private static void whenSubjects() {
        eq(TenOpsKt.stringWhen("first"), 1);
        eq(TenOpsKt.stringWhen(new String("second")), 1);
        eq(TenOpsKt.stringWhen("\u0000\ud800"), 2);
        eq(TenOpsKt.stringWhen("other"), 3);
        for (String value : new String[] {"first", "second", "other"}) {
            Trace.trace = "";
            Trace.value = value;
            eq(TenOpsKt.stringWhenTrace(TRACE), value.equals("other") ? "fallback" : "matched");
            eq(Trace.trace, value.equals("first") ? "S1M" : value.equals("second") ? "S12M" : "S12F");
        }
        for (boolean value : new boolean[] {false, true}) {
            eq(TenOpsKt.booleanWhen(value), value ? "yes" : "no");
            eq(TenOpsKt.booleanWhenStatement(value), value ? 1 : 0);
        }
        eq(TenOpsKt.longWhen(Long.MIN_VALUE), "minimum");
        eq(TenOpsKt.longWhen(Long.MAX_VALUE), "maximum");
        eq(TenOpsKt.longWhen(0L), "other");
    }

    private static void subjectless() {
        for (boolean first : new boolean[] {false, true}) {
            for (boolean second : new boolean[] {false, true}) {
                Trace.trace = "";
                Trace.firstCondition = first;
                Trace.secondCondition = second;
                eq(TenOpsKt.subjectlessWhen(TRACE), first ? "first" : second ? "second" : "fallback");
                eq(Trace.trace, first ? "a1" : second ? "ab2" : "abF");
            }
        }
        for (int value : new int[] {Integer.MIN_VALUE, -1, 0, 1, Integer.MAX_VALUE}) eq(TenOpsKt.subjectlessStatement(value), Integer.compare(value, 0));
    }

    private static void templates() {
        for (String value : new String[] {null, "", "\u0000\ud800", "value"}) {
            eq(TenOpsKt.nullableTemplate(value), (value == null ? "null" : value) + ":null");
            Trace.trace = "";
            eq(TenOpsKt.nullableConcat(TRACE, value), "prefix:" + value + ":null");
            eq(Trace.trace, "R");
            for (long number : new long[] {Long.MIN_VALUE, -1L, 0L, Long.MAX_VALUE}) {
                eq(TenOpsKt.template(value, number), "before:" + (value == null ? "<null>" : value) + ":" + (number + 1L) + ":{}:after");
            }
        }
        Trace.trace = "";
        eq(TenOpsKt.templateTrace(TRACE), "first|second|fallback");
        eq(Trace.trace, "12F");
        eq(TenOpsKt.templateEscaped(), "${literal}:\u0000\ud800:\udfff");
        eq(TenOpsKt.templateWideCall(), "answer=" + Long.MAX_VALUE);
        eq(TenOpsKt.wideCalls(), -2L);
        for (String text : new String[] {null, "present"}) {
            for (long count : new long[] {Long.MIN_VALUE, 0L, Long.MAX_VALUE}) {
                Frames frames = new Frames(text, count);
                eq(frames.getVerse(0), (count + 1L) + ":" + (text == null ? "absent" : text));
                eq(frames.getVerseNumberText(38), "39");
                eq(frames.same(count, 17L, "name"), Math.max(count, 17L) + ":" + (text == null ? "name" : text));
                eq(frames.getCount(), count);
            }
        }
    }

    public static void main(String[] args) throws Exception {
        observe("long_scalars", TenCheck::longs);
        observe("public_constructor_val", TenCheck::publicProperties);
        observe("data_classes", TenCheck::data);
        observe("nullable_references", TenCheck::nullable);
        observe("safe_reference_calls", TenCheck::safe);
        observe("elvis", TenCheck::elvis);
        observe("string_lists", TenCheck::lists);
        observe("string_boolean_when", TenCheck::whenSubjects);
        observe("subjectless_when", TenCheck::subjectless);
        observe("braced_templates", TenCheck::templates);
    }
}
