import anonymousgeneric.*;
import java.lang.reflect.*;
public class Check {
    @SuppressWarnings({"rawtypes", "unchecked"})
    public static void main(String[] args) throws Exception {
        Base<Integer> ints = SourceKt.ints(11);
        if (ints.getSeed() != 11 || ints.apply(7) != 18) throw new AssertionError("primitive base/bridge");
        if (SourceKt.longMapper().map(10L) != 17L) throw new AssertionError("wide bridge");
        Base<String> strings = SourceKt.strings("!");
        if (!strings.getSeed().equals("!") || !strings.apply("x").equals("x!")) throw new AssertionError("inherited substitution/super");
        if (!((Mapper<String>) strings).map("y").equals("y!")) throw new AssertionError("additional generic interface");
        java.util.List<String> list = java.util.Arrays.asList("a", "b");
        Mapper<java.util.List<String>> listMapper = SourceKt.listMapper();
        if (listMapper.map(list) != list) throw new AssertionError("nested parameter substitution");
        boolean nestedSignature = false;
        for (Method m : listMapper.getClass().getDeclaredMethods()) if (!m.isBridge() && m.getGenericReturnType().getTypeName().equals("java.util.List<java.lang.String>")) nestedSignature = true;
        if (!nestedSignature) throw new AssertionError("nested method signature");
        if (SourceKt.boxedMapper().map(null) != null || SourceKt.boxedMapper().map(3) != 3) throw new AssertionError("boxed nullable substitution");
        if (SourceKt.nullable().map(null) != null) throw new AssertionError("nullable bridge");
        if (ints == SourceKt.ints(11)) throw new AssertionError("fresh allocation");
        try { ((Base) ints).apply("bad"); throw new AssertionError("bridge cast missing"); } catch (ClassCastException expected) {}
        try { ((Mapper) strings).map(null); throw new AssertionError("nonnull check missing"); } catch (NullPointerException expected) {}
        if (!ints.getClass().getGenericSuperclass().getTypeName().equals("anonymousgeneric.Base<java.lang.Integer>")) throw new AssertionError("primitive generic superclass");
        if (!strings.getClass().getGenericSuperclass().getTypeName().equals("anonymousgeneric.Middle<java.lang.String>")) throw new AssertionError("generic superclass");
        if (!strings.getClass().getGenericInterfaces()[0].getTypeName().equals("anonymousgeneric.Mapper<java.lang.String>")) throw new AssertionError("generic interface");
        boolean bridge = false;
        for (Method m : ints.getClass().getDeclaredMethods()) if (m.isBridge() && m.isSynthetic() && m.getName().equals("apply")) bridge = true;
        if (!bridge) throw new AssertionError("bridge flags");
        if (ints.getClass().getDeclaredMethod("apply", int.class).getReturnType() != Integer.class) throw new AssertionError("boxed specialized result descriptor");
        System.out.println("OK");
    }
}
