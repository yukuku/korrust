package constructorvarargscheck;
import constructorvarargs.*;
import java.util.Arrays;
public class Calls {
    public static void main(String[] args) throws Exception {
        if (!Strings.class.getConstructor(String.class, String[].class).isVarArgs()) throw new AssertionError("constructor ACC_VARARGS");
        int immutable = Strings.class.getDeclaredField("values").getModifiers();
        int replaceable = Mutable.class.getDeclaredField("values").getModifiers();
        if (!java.lang.reflect.Modifier.isPrivate(immutable) || !java.lang.reflect.Modifier.isFinal(immutable)) throw new AssertionError("val field flags");
        if (!java.lang.reflect.Modifier.isPrivate(replaceable) || java.lang.reflect.Modifier.isFinal(replaceable)) throw new AssertionError("var field flags");
        if (Delegating.class.getConstructor(int.class).isVarArgs()) throw new AssertionError("secondary flags");
        if (Default.class.getConstructor(int.class, int[].class, int.class, kotlin.jvm.internal.DefaultConstructorMarker.class).isVarArgs()) throw new AssertionError("default bridge flags");
        if (LibraryKt.empty().getValues().length != 0) throw new AssertionError("empty source");
        if (!Arrays.equals(LibraryKt.packed().getValues(), new String[]{"a", "bb"})) throw new AssertionError("packed source");
        String[] input = {"a", "b"};
        Strings spread = LibraryKt.spread(input);
        input[0] = "changed";
        if (!Arrays.equals(spread.getValues(), new String[]{"before", "a", "b", "after"})) throw new AssertionError("spread copy");
        String[] direct = {"one"};
        Strings java = new Strings("j", direct);
        if (java.getValues() != direct || !java.getPrefix().equals("j")) throw new AssertionError("Java array identity");
        if (new Strings("j").getValues().length != 0) throw new AssertionError("Java varargs flag");
        if (!Arrays.equals(LibraryKt.primitive(new int[]{1,2,3}).getValues(), new int[]{0,1,2,3,4})) throw new AssertionError("primitive");
        if (!Arrays.equals(LibraryKt.wide().getValues(), new long[]{4294967298L,4294967299L})) throw new AssertionError("wide");
        if (!Arrays.equals(LibraryKt.nullable().getValues(), new String[]{null,"n"})) throw new AssertionError("nullable");
        if (!Arrays.equals(LibraryKt.generic().getValues(), new String[]{"x","yy"})) throw new AssertionError("generic source");
        if (!Arrays.equals(LibraryKt.genericInts().getValues(), new Integer[]{1,2})) throw new AssertionError("generic boxed primitive");
        if (!Arrays.equals(new Generic<String>("j","k").getValues(), new String[]{"j","k"})) throw new AssertionError("generic Java");
        Mutable mutable = new Mutable("a");
        mutable.setValues(direct);
        if (mutable.getValues() != direct) throw new AssertionError("mutable property");
        if (new HiddenAfter("a", "b").count() != 2 || new HiddenBefore("a").count() != 1) throw new AssertionError("private properties and modifier order");
        if (new Bare(1,2,3).getCount() != 3) throw new AssertionError("bare parameter");
        if (LibraryKt.defaults().getPrefix() != 7 || !Arrays.equals(LibraryKt.defaults().getValues(), new int[]{1,2})) throw new AssertionError("leading default");
        if (!Arrays.equals(new Child().getValues(), new int[]{2,3})) throw new AssertionError("super delegation");
        if (!Arrays.equals(new SpreadChild(new int[]{7,8}).getValues(), new int[]{7,8})) throw new AssertionError("spread super delegation");
        if (!Arrays.equals(new Delegating(true, new int[]{7,8}).getValues(), new int[]{7,8})) throw new AssertionError("spread this delegation");
        if (!Arrays.equals(new Delegating(4).getValues(), new int[]{4,5,6})) throw new AssertionError("this delegation");
        if (!Arrays.equals(LibraryKt.effects().getValues(), new int[]{1,2,3,4}) || Trace.INSTANCE.getLog() != 1234) throw new AssertionError("evaluation order");
        boolean nullRejected = false;
        try { new Strings("j", (String[])null); } catch (NullPointerException expected) { nullRejected = true; }
        if (!nullRejected) throw new AssertionError("nonnull array");
        System.out.println("constructor-varargs-ok");
    }
}
