import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.EnumMap;
import java.util.EnumSet;
import java.util.HexFormat;
import java.util.ListIterator;
import java.util.function.Function;
import java.util.function.Supplier;
import kotlin.enums.EnumEntries;
import korrust.probes.enums.CollisionEnum;
import korrust.probes.enums.LargeEnum;
import korrust.probes.enums.EmptyBodylessEnum;
import korrust.probes.enums.EmptyEnum;
import korrust.probes.enums.EmptySemicolonEnum;
import korrust.probes.enums.EnumOrder;
import korrust.probes.enums.InternalEnum;
import korrust.probes.enums.SingleEnum;
import yuku.alkitab.base.actionmode.RibkaEligibility;

public final class EnumsCheck {
    private static final MessageDigest DIGEST;
    private static int checks;
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception error) { throw new ExceptionInInitializerError(error); }
    }
    private static void record(Object actual, Object expected) {
        if (!actual.equals(expected)) throw new AssertionError("Check " + checks + ": " + actual + " != " + expected);
        byte[] bytes = actual.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8);
        for (int shift = 24; shift >= 0; shift -= 8) DIGEST.update((byte)(bytes.length >>> shift));
        DIGEST.update(bytes);
        checks++;
    }
    private static String failure(Runnable action) {
        try { action.run(); return "success"; }
        catch (RuntimeException error) { return error.getClass().getName(); }
    }
    private static Object roundTrip(Object value) throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (ObjectOutputStream output = new ObjectOutputStream(bytes)) { output.writeObject(value); }
        try (ObjectInputStream input = new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))) { return input.readObject(); }
    }
    private static <E extends Enum<E>> void check(Class<E> type, String[] names, Supplier<E[]> values,
            Function<String, E> valueOf, Supplier<EnumEntries<E>> getEntries) throws Exception {
        record(type.isEnum(), true);
        record(Modifier.isPublic(type.getModifiers()), true);
        record(Modifier.isFinal(type.getModifiers()), true);
        record(type.getGenericSuperclass().getTypeName(), "java.lang.Enum<" + type.getName() + ">");
        record(type.getMethod("getEntries").getGenericReturnType().getTypeName(), "kotlin.enums.EnumEntries<" + type.getName() + ">");
        record(type.getMethod("values").getReturnType(), values.get().getClass());
        record(type.getMethod("valueOf", String.class).getReturnType(), type);
        record(type.getDeclaredConstructors().length, 1);
        record(Arrays.asList(type.getDeclaredConstructors()[0].getParameterTypes()), Arrays.asList(String.class, int.class));
        record(Modifier.isPrivate(type.getDeclaredConstructors()[0].getModifiers()), true);
        E[] first = values.get();
        E[] second = values.get();
        record(first == second, false);
        record(first.length, names.length);
        record(Arrays.equals(first, second), true);
        record(Arrays.equals(type.getEnumConstants(), first), true);
        EnumEntries<E> entries = getEntries.get();
        record(entries == getEntries.get(), true);
        record(entries.size(), names.length);
        record(entries.isEmpty(), names.length == 0);
        record(entries.equals(Arrays.asList(first)), true);
        record(entries.hashCode(), Arrays.asList(first).hashCode());
        EnumSet<E> set = EnumSet.allOf(type);
        EnumMap<E, Integer> map = new EnumMap<>(type);
        for (int i = 0; i < names.length; i++) {
            E value = first[i];
            record(value.name(), names[i]);
            record(value.toString(), names[i]);
            record(value.ordinal(), i);
            record(value.getDeclaringClass(), type);
            record(value.getClass(), type);
            record(valueOf.apply(names[i]) == value, true);
            record(Enum.valueOf(type, names[i]) == value, true);
            record(type.getField(names[i]).get(null) == value, true);
            Field field = type.getField(names[i]);
            record(field.isEnumConstant(), true);
            record(field.getModifiers(), Modifier.PUBLIC | Modifier.STATIC | Modifier.FINAL | 0x4000);
            record(field.getType(), type);
            record(entries.get(i) == value, true);
            record(entries.contains(value), true);
            record(entries.indexOf(value), i);
            record(entries.lastIndexOf(value), i);
            record(roundTrip(value) == value, true);
            map.put(value, i);
            for (int j = 0; j < names.length; j++) {
                if (names.length <= 16 || j == 0 || j == i || j == names.length - 1) record(value.compareTo(first[j]), i - j);
            }
        }
        record(Arrays.asList(set.toArray()), Arrays.asList(first));
        record(Arrays.asList(map.keySet().toArray()), Arrays.asList(first));
        record(entries.contains(null), false);
        record(entries.indexOf(null), -1);
        record(entries.lastIndexOf(null), -1);
        record(entries.contains("missing"), false);
        record(entries.indexOf("missing"), -1);
        E element = first.length == 0 ? null : first[0];
        record(failure(() -> entries.add(element)), "java.lang.UnsupportedOperationException");
        record(failure(() -> entries.set(0, element)), "java.lang.UnsupportedOperationException");
        record(failure(() -> entries.remove(0)), "java.lang.UnsupportedOperationException");
        record(failure(entries::clear), "java.lang.UnsupportedOperationException");
        if (first.length != 0) {
            ListIterator<E> iterator = entries.listIterator();
            record(iterator.next() == first[0], true);
            record(failure(iterator::remove), "java.lang.UnsupportedOperationException");
            first[0] = null;
            record(values.get()[0] == second[0], true);
            record(entries.get(0) == second[0], true);
            record(type.getEnumConstants()[0] == second[0], true);
        }
        for (int index : new int[] {-1, names.length, Integer.MIN_VALUE, Integer.MAX_VALUE}) {
            record(failure(() -> entries.get(index)), "java.lang.IndexOutOfBoundsException");
        }
        for (String name : new String[] {"", "missing", " ", names.length == 0 ? "Only" : names[0] + " "}) {
            record(failure(() -> valueOf.apply(name)), "java.lang.IllegalArgumentException");
        }
        record(failure(() -> valueOf.apply(null)), "java.lang.NullPointerException");
    }
    public static void main(String[] args) throws Exception {
        record(RibkaEligibility.None, RibkaEligibility.valueOf("None"));
        record(RibkaEligibility.Main.ordinal(), 1);
        record(RibkaEligibility.Split.ordinal(), 2);
        record(EnumOrder.Alpha, EnumOrder.valueOf("Alpha"));
        record(EnumOrder.MIDDLE.ordinal(), 1);
        record(EnumOrder.Zed.ordinal(), 2);
        check(RibkaEligibility.class, new String[] {"None", "Main", "Split"}, RibkaEligibility::values, RibkaEligibility::valueOf, RibkaEligibility::getEntries);
        check(EnumOrder.class, new String[] {"Alpha", "MIDDLE", "Zed"}, EnumOrder::values, EnumOrder::valueOf, EnumOrder::getEntries);
        check(SingleEnum.class, new String[] {"Only"}, SingleEnum::values, SingleEnum::valueOf, SingleEnum::getEntries);
        check(InternalEnum.class, new String[] {"FIRST", "SECOND"}, InternalEnum::values, InternalEnum::valueOf, InternalEnum::getEntries);
        check(CollisionEnum.class, new String[] {"values", "valueOf", "CollisionEnum"}, CollisionEnum::values, CollisionEnum::valueOf, CollisionEnum::getEntries);
        String[] largeNames = new String[1024];
        for (int i = 0; i < largeNames.length; i++) largeNames[i] = "ENTRY_" + i;
        check(LargeEnum.class, largeNames, LargeEnum::values, LargeEnum::valueOf, LargeEnum::getEntries);
        check(EmptyEnum.class, new String[0], EmptyEnum::values, EmptyEnum::valueOf, EmptyEnum::getEntries);
        check(EmptySemicolonEnum.class, new String[0], EmptySemicolonEnum::values, EmptySemicolonEnum::valueOf, EmptySemicolonEnum::getEntries);
        check(EmptyBodylessEnum.class, new String[0], EmptyBodylessEnum::values, EmptyBodylessEnum::valueOf, EmptyBodylessEnum::getEntries);
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}
