package next10.shorts;
import java.util.Arrays;
public final class Calls {
    private static int checks;
    private static void require(boolean value) { checks++; if (!value) throw new AssertionError("check=" + checks); }
    private static void throwsType(Class<? extends Throwable> type, Runnable action) {
        checks++;
        try { action.run(); } catch (Throwable error) { if (type.isInstance(error)) return; throw new AssertionError(error); }
        throw new AssertionError("missing " + type);
    }
    public static void main(String[] args) {
        require(ValuesKt.shortSum((short)32767, (short)32767) == 65534);
        require(ValuesKt.shortUnary((short)-32768) == 32768);
        require(ValuesKt.byteUnary((byte)-128) == -128);
        require(ValuesKt.mixed(Integer.MAX_VALUE, 1L) == 2147483648L);
        require(ValuesKt.mixed(-1, Long.MIN_VALUE) == Long.MAX_VALUE);
        require(ValuesKt.reverse(Long.MIN_VALUE, 1) == Long.MAX_VALUE);
        require(ValuesKt.product((short)-32768, 65536L) == -2147483648L);
        require(ValuesKt.quotient(Long.MIN_VALUE, (short)-1) == Long.MIN_VALUE);
        throwsType(ArithmeticException.class, () -> ValuesKt.quotient(1L, (short)0));
        require(ValuesKt.remainder((short)-32768, 3) == -2);
        require(ValuesKt.shortOrder((short)-32768, Integer.MAX_VALUE));
        require(ValuesKt.longOrder(Integer.MIN_VALUE, Long.MIN_VALUE));
        require(!ValuesKt.longOrder(Integer.MAX_VALUE, Long.MAX_VALUE));
        require(ValuesKt.byteSum((byte)127, (short)32767) == 32894);
        require(ValuesKt.floating((short)-2, 0.5) == -1.5);
        require(ValuesKt.narrow(65535L) == -1);
        require(ValuesKt.narrow(Long.MAX_VALUE) == -1);
        require(ValuesKt.byteNarrow((short)32767) == -1);
        require(ValuesKt.charNarrow((short)-1) == '\uffff');
        require(ValuesKt.widen((short)-32768) == -32768L);
        require(ValuesKt.boxed((short)-32768).shortValue() == -32768);
        require(ValuesKt.unbox(Short.valueOf((short)32767)) == 32767);
        throwsType(NullPointerException.class, () -> ValuesKt.unbox(null));
        require(ValuesKt.contextual() == -32768);
        require(ValuesKt.contextualNullable().shortValue() == 32767);
        require(ValuesKt.text((short)-32768).equals("v=-32768"));
        require(ValuesKt.equality((short)32767, (short)32767));
        require(!ValuesKt.equality((short)32767, (short)-1));
        require(ValuesKt.fromInt(32768) == -32768);
        require(Arrays.equals(ValuesKt.empty(3), new short[]{0, 0, 0}));
        throwsType(NegativeArraySizeException.class, () -> ValuesKt.empty(-1));
        require(Arrays.equals(ValuesKt.initialized(4), new short[]{0, 32767, -2, 32765}));
        short[] values = {-32768, 32767};
        require(ValuesKt.get(values, 0) == -32768);
        require(ValuesKt.get(values, 1) == 32767);
        require(ValuesKt.set(values, 1, (short)-12345) == -12345 && values[1] == -12345);
        ValuesKt.literalStore(values);
        require(Arrays.equals(values, new short[]{-32768, 32767}));
        throwsType(ArrayIndexOutOfBoundsException.class, () -> ValuesKt.get(values, -1));
        throwsType(ArrayIndexOutOfBoundsException.class, () -> ValuesKt.set(values, 2, (short)0));
        require(ValuesKt.total(values) == -1L);
        require(ValuesKt.total(new short[0]) == 0L);
        require(Arrays.equals(ValuesKt.pack((short)-1, (short)32767), new short[]{-1, 32767}));
        require(Arrays.equals(ValuesKt.spread(values), new short[]{-32768, -32768, 32767, 32767}));
        require(Arrays.equals(ValuesKt.factory(), new short[]{-32768, 0, 32767}));
        Effects.trace = "";
        require(ValuesKt.index(values) == -32768 && Effects.trace.equals("I"));
        Effects.trace = "";
        require(Arrays.equals(ValuesKt.ordered(values), new short[]{1, -32768, 32767, 2}) && Effects.trace.equals("1A2"));
        Effects.trace = "";
        require(Arrays.equals(ValuesKt.initializeEffects(3), new short[]{0, 1, 2}) && Effects.trace.equals("012"));
        Effects.trace = "";
        require(ValuesKt.initializeEffects(0).length == 0 && Effects.trace.isEmpty());
        Effects.trace = "";
        throwsType(NegativeArraySizeException.class, () -> ValuesKt.initializeEffects(-1));
        require(Effects.trace.isEmpty());
        require(ValuesKt.javaArray(values) == values);
        Holder holder = new Holder((short)-32768, values);
        require(holder.getScalar() == -32768 && holder.getValues() == values);
        require(Arrays.equals(ValuesKt.imported(values), new short[]{-1, -32768, 32767}));
        require(ValuesKt.importedScalar((short)-32768) == -32768);
        require(Arrays.equals(AliasedFactoryKt.factoryAlias(), new short[]{-32768, 32767}));
        require(ValuesKt.nullable(values) == values && ValuesKt.nullable(null) == null);
        require(ValuesKt.cast(values) == values);
        throwsType(ClassCastException.class, () -> ValuesKt.cast(new byte[0]));
        System.out.println("checks=" + checks);
    }
}
