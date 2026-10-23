import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Objects;
import korrust.probes.objects.BodylessObject;
import korrust.probes.objects.CollisionObject;
import korrust.probes.objects.EmptyObject;
import korrust.probes.objects.InternalObject;
import korrust.probes.objects.LargeObject;
import korrust.probes.objects.ObjectLiterals;
import korrust.probes.objects.ObjectTree;
import yuku.alkitab.base.util.RequestCodes;

public final class ObjectsCheck {
    private static final MessageDigest DIGEST;
    private static int checks;
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception error) { throw new ExceptionInInitializerError(error); }
    }
    private static void record(Object actual, Object expected) {
        if (!Objects.equals(actual, expected)) throw new AssertionError("Check " + checks + ": " + actual + " != " + expected);
        byte[] bytes = String.valueOf(actual).getBytes(java.nio.charset.StandardCharsets.UTF_8);
        for (int shift = 24; shift >= 0; shift -= 8) DIGEST.update((byte)(bytes.length >>> shift));
        DIGEST.update(bytes);
        checks++;
    }
    private static void checkObject(Class<?> type, Object instance, Class<?> parent, int constants, Class<?>... children) throws Exception {
        record(instance != null, true);
        record(instance.getClass(), type);
        record(type.getSuperclass(), Object.class);
        record(type.getModifiers(), Modifier.PUBLIC | Modifier.FINAL | (parent == null ? 0 : Modifier.STATIC));
        record(type.isEnum(), false);
        record(type.isInterface(), false);
        record(type.isAnonymousClass(), false);
        record(type.isLocalClass(), false);
        record(type.isMemberClass(), parent != null);
        record(type.getDeclaringClass(), parent);
        record(type.getEnclosingClass(), parent);
        record(type.getCanonicalName(), type.getName().replace('$', '.'));
        String[] parts = type.getName().split("[.$]");
        record(type.getSimpleName(), parts[parts.length - 1]);
        record(type.getDeclaredConstructors().length, 1);
        record(type.getDeclaredConstructors()[0].getParameterCount(), 0);
        record(type.getDeclaredConstructors()[0].getModifiers(), Modifier.PRIVATE);
        record(type.getDeclaredMethods().length, 0);
        record(type.getDeclaredFields().length, constants + 1);
        Field singleton = type.getField("INSTANCE");
        record(singleton.getType(), type);
        record(singleton.getModifiers(), Modifier.PUBLIC | Modifier.STATIC | Modifier.FINAL);
        record(singleton.get(null) == instance, true);
        record(singleton.get(null) == singleton.get(null), true);
        record(singleton.isSynthetic(), false);
        String[] actual = Arrays.stream(type.getDeclaredClasses()).map(Class::getName).sorted().toArray(String[]::new);
        String[] expected = Arrays.stream(children).map(Class::getName).sorted().toArray(String[]::new);
        record(Arrays.asList(actual), Arrays.asList(expected));
    }
    private static void constant(Class<?> owner, String name, Class<?> type, Object expected, Object direct) throws Exception {
        Field field = owner.getDeclaredField(name);
        record(field.getType(), type);
        record(field.getModifiers(), Modifier.PUBLIC | Modifier.STATIC | Modifier.FINAL);
        record(field.isSynthetic(), false);
        record(field.get(null), expected);
        record(direct, expected);
        record(field.get(null), direct);
        if (expected instanceof String text) {
            String actual = (String)field.get(null);
            record(actual.length(), text.length());
            for (int i = 0; i < text.length(); i++) record((int)actual.charAt(i), (int)text.charAt(i));
        }
    }
    public static void main(String[] args) throws Exception {
        checkObject(RequestCodes.FromActivity.class, RequestCodes.FromActivity.INSTANCE, RequestCodes.class, 6);
        checkObject(RequestCodes.class, RequestCodes.INSTANCE, null, 0, RequestCodes.FromActivity.class);
        constant(RequestCodes.FromActivity.class, "Goto", int.class, 1, RequestCodes.FromActivity.Goto);
        constant(RequestCodes.FromActivity.class, "TextAppearanceGetFonts", int.class, 9, RequestCodes.FromActivity.TextAppearanceGetFonts);
        constant(RequestCodes.FromActivity.class, "TextAppearanceCustomColors", int.class, 10, RequestCodes.FromActivity.TextAppearanceCustomColors);
        constant(RequestCodes.FromActivity.class, "EditNote1", int.class, 11, RequestCodes.FromActivity.EditNote1);
        constant(RequestCodes.FromActivity.class, "EditNote2", int.class, 12, RequestCodes.FromActivity.EditNote2);
        constant(RequestCodes.FromActivity.class, "PermissionSettings", int.class, 9970, RequestCodes.FromActivity.PermissionSettings);
        checkObject(ObjectLiterals.class, ObjectLiterals.INSTANCE, null, 17);
        constant(ObjectLiterals.class, "MIN", int.class, Integer.MIN_VALUE, ObjectLiterals.MIN);
        constant(ObjectLiterals.class, "MIN_TYPED", int.class, Integer.MIN_VALUE, ObjectLiterals.MIN_TYPED);
        constant(ObjectLiterals.class, "MAX", int.class, Integer.MAX_VALUE, ObjectLiterals.MAX);
        constant(ObjectLiterals.class, "NEGATIVE", int.class, -1, ObjectLiterals.NEGATIVE);
        constant(ObjectLiterals.class, "ZERO", int.class, 0, ObjectLiterals.ZERO);
        constant(ObjectLiterals.class, "TRUE", boolean.class, true, ObjectLiterals.TRUE);
        constant(ObjectLiterals.class, "FALSE", boolean.class, false, ObjectLiterals.FALSE);
        constant(ObjectLiterals.class, "NUL", char.class, (char)0, ObjectLiterals.NUL);
        constant(ObjectLiterals.class, "HIGH", char.class, (char)0xd800, ObjectLiterals.HIGH);
        constant(ObjectLiterals.class, "LOW", char.class, (char)0xdfff, ObjectLiterals.LOW);
        constant(ObjectLiterals.class, "LAST", char.class, (char)0xffff, ObjectLiterals.LAST);
        constant(ObjectLiterals.class, "NEWLINE", char.class, '\n', ObjectLiterals.NEWLINE);
        constant(ObjectLiterals.class, "QUOTE", char.class, '\'', ObjectLiterals.QUOTE);
        constant(ObjectLiterals.class, "BACKSLASH", char.class, '\\', ObjectLiterals.BACKSLASH);
        constant(ObjectLiterals.class, "EMPTY", String.class, "", ObjectLiterals.EMPTY);
        constant(ObjectLiterals.class, "TEXT", String.class, new String(new char[] {'A', 0, 0xd800, 'Z', 0xdc00, 0xffff, 0x6f22, 0xd83d, 0xde00}), ObjectLiterals.TEXT);
        constant(ObjectLiterals.class, "ESCAPES", String.class, "\\\t\b\n\r'\"$", ObjectLiterals.ESCAPES);
        checkObject(ObjectTree.Left.Leaf.class, ObjectTree.Left.Leaf.INSTANCE, ObjectTree.Left.class, 1);
        checkObject(ObjectTree.Right.class, ObjectTree.Right.INSTANCE, ObjectTree.class, 1);
        checkObject(ObjectTree.Empty.class, ObjectTree.Empty.INSTANCE, ObjectTree.class, 0);
        checkObject(ObjectTree.Left.class, ObjectTree.Left.INSTANCE, ObjectTree.class, 1, ObjectTree.Left.Leaf.class);
        checkObject(ObjectTree.class, ObjectTree.INSTANCE, null, 1, ObjectTree.Left.class, ObjectTree.Right.class, ObjectTree.Empty.class);
        record((Object)ObjectTree.INSTANCE == ObjectTree.Left.INSTANCE, false);
        record((Object)ObjectTree.Left.INSTANCE == ObjectTree.Right.INSTANCE, false);
        record((Object)ObjectTree.Left.INSTANCE == ObjectTree.Left.Leaf.INSTANCE, false);
        constant(ObjectTree.class, "ROOT", int.class, 1, ObjectTree.ROOT);
        constant(ObjectTree.Left.class, "KEY", String.class, "left", ObjectTree.Left.KEY);
        constant(ObjectTree.Left.Leaf.class, "VALUE", int.class, 11, ObjectTree.Left.Leaf.VALUE);
        constant(ObjectTree.Right.class, "KEY", String.class, "right", ObjectTree.Right.KEY);
        checkObject(InternalObject.class, InternalObject.INSTANCE, null, 3, InternalObject.Child.class);
        checkObject(InternalObject.Child.class, InternalObject.Child.INSTANCE, InternalObject.class, 1);
        constant(InternalObject.class, "VALUE", int.class, 37, InternalObject.VALUE);
        constant(InternalObject.class, "PUBLIC", int.class, 41, InternalObject.PUBLIC);
        constant(InternalObject.class, "INTERNAL", int.class, 43, InternalObject.INTERNAL);
        constant(InternalObject.Child.class, "FLAG", boolean.class, true, InternalObject.Child.FLAG);
        Class<?> self = Class.forName("korrust.probes.objects.CollisionObject$CollisionObject");
        checkObject(CollisionObject.class, CollisionObject.INSTANCE, null, 0, self);
        checkObject(self, self.getField("INSTANCE").get(null), CollisionObject.class, 1);
        constant(self, "CollisionObject", int.class, 7, 7);
        checkObject(EmptyObject.class, EmptyObject.INSTANCE, null, 0);
        checkObject(BodylessObject.class, BodylessObject.INSTANCE, null, 0);
        checkObject(LargeObject.class, LargeObject.INSTANCE, null, 1024);
        for (int i = 0; i < 1024; i++) constant(LargeObject.class, "VALUE_" + i, int.class, i, i);
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}
