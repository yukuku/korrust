import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Objects;

public final class InterfaceAssertions {
    private static final MessageDigest DIGEST;
    private static int checks;
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception error) { throw new ExceptionInInitializerError(error); }
    }
    public static void record(Object actual, Object expected) {
        if (!Objects.equals(actual, expected)) throw new AssertionError("Check " + checks + ": " + actual + " != " + expected);
        String text = String.valueOf(actual);
        for (int i = 0; i < text.length(); i++) { DIGEST.update((byte)(text.charAt(i) >>> 8)); DIGEST.update((byte)text.charAt(i)); }
        DIGEST.update((byte)0xff);
        checks++;
    }
    public static String failure(Runnable action) {
        try { action.run(); return "success"; }
        catch (RuntimeException error) { return error.getClass().getName() + ":" + error.getMessage(); }
    }
    public static Object call(Method method, Object receiver, Object... arguments) {
        try { return method.invoke(receiver, arguments); }
        catch (InvocationTargetException error) {
            if (error.getCause() instanceof RuntimeException failure) throw failure;
            throw new AssertionError(error.getCause());
        }
        catch (ReflectiveOperationException error) { throw new AssertionError(error); }
    }
    public static Object artifacts(Class<?> outer, String countName, String labelName, String nestedName,
            String floatName, String arrayName, String emptyName, String[] labels, int emptyCount, int zeroIndex) throws Exception {
        record(outer.getModifiers(), Modifier.PUBLIC | Modifier.ABSTRACT | Modifier.INTERFACE);
        Class<?> companion = Class.forName(outer.getName() + "$Companion", false, outer.getClassLoader());
        Field arrayField = companion.getDeclaredField(arrayName);
        record(arrayField.getModifiers(), Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
        record(arrayField.getType(), String[].class);
        arrayField.setAccessible(true);
        String[] array = (String[])arrayField.get(null);
        record(Arrays.asList(array), Arrays.asList(labels));
        Field companionField = outer.getField("Companion");
        record(companionField.getModifiers(), Modifier.PUBLIC | Modifier.STATIC | Modifier.FINAL);
        Object singleton = companionField.get(null);
        record(singleton.getClass(), companion);
        Field privateSingleton = companion.getDeclaredField("$$INSTANCE");
        privateSingleton.setAccessible(true);
        record(privateSingleton.get(null) == singleton, true);
        record(privateSingleton.getModifiers(), Modifier.STATIC | Modifier.FINAL | 0x1000);
        record(companion.getDeclaredConstructors().length, 1);
        record(companion.getDeclaredConstructors()[0].getModifiers(), Modifier.PRIVATE);
        record(companion.getDeclaredConstructors()[0].getParameterCount(), 0);
        record(companion.getDeclaringClass(), outer);
        record(companion.getSimpleName(), "Companion");
        Method accessor = companion.getDeclaredMethod("access$get" + arrayName + "$p");
        record(accessor.isSynthetic(), true);
        record(accessor.getModifiers(), Modifier.PUBLIC | Modifier.STATIC | Modifier.FINAL | 0x1000);
        record(call(accessor, null) == array, true);
        Method getter = companion.getDeclaredMethod("get" + emptyName);
        record(getter.getReturnType(), outer);
        Object empty = call(getter, singleton);
        record(empty == call(getter, singleton), true);
        Field emptyField = companion.getDeclaredField(emptyName);
        record(emptyField.getModifiers(), Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
        emptyField.setAccessible(true);
        record(emptyField.get(null) == empty, true);
        record(call(outer.getMethod(countName), empty), emptyCount);
        record(empty.getClass().isAnonymousClass(), true);
        record(empty.getClass().getSimpleName(), "");
        record(empty.getClass().getCanonicalName(), null);
        record(empty.getClass().getDeclaringClass(), null);
        record(empty.getClass().getEnclosingClass(), companion);
        record(Arrays.asList(empty.getClass().getInterfaces()), Arrays.asList(outer));
        Class<?> nested = Class.forName(outer.getName() + "$" + nestedName);
        record(nested.getDeclaringClass(), outer);
        record(nested.getModifiers(), Modifier.PUBLIC | Modifier.STATIC | Modifier.ABSTRACT | Modifier.INTERFACE);
        record(Arrays.asList(nested.getInterfaces()), Arrays.asList(outer));
        record(nested.getMethod(floatName, int.class).getReturnType(), float.class);
        for (Class<?> type : new Class<?>[] {outer, nested}) {
            Class<?> defaults = Class.forName(type.getName() + "$DefaultImpls");
            record(defaults.getDeclaringClass(), type);
            record(defaults.getModifiers(), Modifier.PUBLIC | Modifier.STATIC | Modifier.FINAL);
            record(defaults.getDeclaredConstructors().length, 0);
            record(defaults.getDeclaredMethods().length, 1);
            Method method = defaults.getDeclaredMethod(labelName, type, int.class);
            record(method.getReturnType(), String.class);
            record(Modifier.isStatic(method.getModifiers()), true);
        }
        String before = array[0];
        try {
            array[0] = "mutated\u0000\ud800";
            record(call(outer.getMethod(labelName, int.class), empty, zeroIndex), array[0]);
            record(call(accessor, null) == array, true);
        } finally { array[0] = before; }
        record(Arrays.asList(array), Arrays.asList(labels));
        return empty;
    }
    public static void finish() {
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}
