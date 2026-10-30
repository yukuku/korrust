package annotation17check;

import annotation17.*;
import annotation17java.*;
import annotation17shadow.*;
import java.lang.annotation.*;
import java.lang.reflect.*;
import java.util.Arrays;

public final class ReflectionCheck {
    private static int count;
    private static void check(boolean value) {
        count++;
        if (!value) throw new AssertionError("check " + count);
    }
    public static void main(String[] args) throws Exception {
        check(Rich.class.isAnnotation());
        check(Rich.class.getDeclaredConstructors().length == 0);
        check(Rich.class.getInterfaces().length == 1 && Rich.class.getInterfaces()[0] == Annotation.class);
        check(Rich.class.getDeclaredMethods().length == 16);
        for (Method method : Rich.class.getDeclaredMethods()) {
            check(Modifier.isPublic(method.getModifiers()) && Modifier.isAbstract(method.getModifiers()));
            check(method.getParameterCount() == 0);
            check(method.getDefaultValue() != null);
        }
        Rich value = DefaultUse.class.getAnnotation(Rich.class);
        check(value.number() == Long.MIN_VALUE);
        check(Float.floatToRawIntBits(value.ratio()) == 0x80000000);
        check(value.measure() == 1.25);
        check(value.letter() == '\uD800');
        check(value.mode() == Mode.SECOND);
        check(value.type() == Token.class);
        check(Arrays.equals(value.numbers(), new long[] {0L, 1L, -1L}));
        check(Arrays.equals(value.flags(), new boolean[] {true, false}));
        check(Arrays.equals(value.letters(), new char[] {0, 0xD800, 0xDFFF}));
        check(Float.floatToRawIntBits(value.ratios()[0]) == 0x80000000 && value.ratios()[1] == 1.25f);
        check(Double.doubleToRawLongBits(value.measures()[0]) == Long.MIN_VALUE && value.measures()[1] == 2.5);
        check(Arrays.equals(value.names(), new String[] {"", "\u0000", "\uD800", "\uD83D\uDE00"}));
        check(Arrays.equals(value.modes(), new Mode[] {Mode.FIRST, Mode.SECOND}));
        check(Arrays.equals(value.types(), new Class<?>[] {int.class, String[].class, int[].class, kotlin.Unit.class, Token.class}));
        check(value.nested().text().equals("nested") && value.nested().amount() == 17L && value.nested().mode() == Mode.SECOND);
        check(value.children().length == 2 && value.children()[0].text().equals("first") && value.children()[0].amount() == 1L);
        long[] copy = value.numbers(); copy[0] = 99L;
        check(value.numbers()[0] == 0L);
        Nested[] nestedCopy = value.children(); nestedCopy[0] = null;
        check(value.children()[0] != null);
        check(value.equals(DefaultUse.class.getAnnotation(Rich.class)));
        check(value.hashCode() == DefaultUse.class.getAnnotation(Rich.class).hashCode());
        Rich explicit = ExplicitUse.class.getAnnotation(Rich.class);
        check(explicit.number() == Long.MAX_VALUE && explicit.ratio() == 2.5f);
        check(Double.doubleToRawLongBits(explicit.measure()) == Long.MIN_VALUE);
        check(explicit.letter() == 0xFFFF && explicit.mode() == Mode.FIRST && explicit.type() == int[].class);
        check(Arrays.equals(explicit.numbers(), new long[] {7L, -8L}) && explicit.flags().length == 0);
        check(Arrays.equals(explicit.letters(), new char[] {'x', 0}));
        check(Arrays.equals(explicit.ratios(), new float[] {3.5f}));
        check(explicit.measures()[0] == 4.5 && Double.doubleToRawLongBits(explicit.measures()[1]) == Long.MIN_VALUE);
        check(Arrays.equals(explicit.names(), new String[] {"explicit", "\uDFFF"}));
        check(Arrays.equals(explicit.modes(), new Mode[] {Mode.SECOND}));
        check(Arrays.equals(explicit.types(), new Class<?>[] {String.class, long.class, Token[].class}));
        check(explicit.nested().text().equals("outer") && explicit.nested().amount() == 91L && explicit.nested().mode() == Mode.FIRST);
        check(explicit.children().length == 1 && explicit.children()[0].amount() == -3L);
        check(EmptyPack.class.getAnnotation(Pack.class).entries().length == 0);
        check(PositionalPack.class.getAnnotation(Pack.class).entries().length == 2);
        check(NamedPack.class.getAnnotation(Pack.class).entries()[0].amount() == 3L);
        check(SpreadPack.class.getAnnotation(Pack.class).entries()[0].amount() == 4L);
        check(Pack.class.getDeclaredMethod("entries").getDefaultValue() == null);
        check(DefaultUse.class.getAnnotation(BinaryMark.class) == null);
        check(DefaultUse.class.getAnnotation(SourceMark.class) == null);
        check(Rich.class.getAnnotation(java.lang.annotation.Retention.class).value() == RetentionPolicy.RUNTIME);
        check(BinaryMark.class.getAnnotation(java.lang.annotation.Retention.class).value() == RetentionPolicy.CLASS);
        check(SourceMark.class.getAnnotation(java.lang.annotation.Retention.class).value() == RetentionPolicy.SOURCE);
        check(Rich.class.getAnnotation(Meta.class).id().equals("rich"));
        check(Placed.class.getDeclaredConstructors()[0].getParameterAnnotations()[0][0] instanceof Rich);
        check(((Rich) Placed.class.getDeclaredConstructors()[0].getParameterAnnotations()[0][0]).number() == 11L);
        check(Placed.class.getDeclaredField("value").getAnnotation(Rich.class).number() == 12L);
        check(Placed.class.getDeclaredMethod("getValue").getAnnotation(Rich.class).number() == 13L);
        check(Placed.class.getDeclaredMethod("setValue", int.class).getAnnotation(Rich.class).number() == 14L);
        check(((Rich) Placed.class.getDeclaredMethod("setValue", int.class).getParameterAnnotations()[0][0]).number() == 15L);
        check(Placed.class.getDeclaredMethod("getProperty$annotations").getAnnotation(Rich.class).number() == 16L);
        Method method = Placed.class.getDeclaredMethod("method", int.class);
        check(method.getAnnotation(Rich.class).number() == 17L);
        check(((Rich) method.getParameterAnnotations()[0][0]).number() == 18L);
        JavaRich javaDefault = JavaDefaultUse.class.getAnnotation(JavaRich.class);
        check(Arrays.equals(javaDefault.value(), new String[] {"java", "default"}));
        check(javaDefault.number() == Long.MIN_VALUE && Float.floatToRawIntBits(javaDefault.ratio()) == 0x80000000);
        check(Double.doubleToRawLongBits(javaDefault.measure()) == Long.MIN_VALUE && javaDefault.letter() == 0xD800);
        check(javaDefault.type() == int[].class && Arrays.equals(javaDefault.types(), new Class<?>[] {int.class, String[].class}));
        check(javaDefault.mode() == RetentionPolicy.RUNTIME && javaDefault.nested().amount() == 37L);
        check(javaDefault.children()[0].text().equals("child") && javaDefault.children()[0].amount() == 41L);
        JavaRich javaSingle = JavaSingleUse.class.getAnnotation(JavaRich.class);
        check(Arrays.equals(javaSingle.value(), new String[] {"single"}) && javaSingle.number() == 9L);
        check(javaSingle.nested().text().equals("explicit") && javaSingle.nested().amount() == 43L);
        JavaRich javaExplicit = JavaExplicitUse.class.getAnnotation(JavaRich.class);
        check(Arrays.equals(javaExplicit.value(), new String[] {"left", "right"}) && javaExplicit.type() == Token.class);
        check(Arrays.equals(javaExplicit.types(), new Class<?>[] {long.class, Token.class}) && javaExplicit.mode() == RetentionPolicy.CLASS);
        check(javaExplicit.children()[0].amount() == 47L);
        check(JavaEmptyUse.class.getAnnotation(JavaRich.class).value().length == 0);
        check(UserMarked.class.getAnnotation(annotation17shadow.Retention.class).value() == 7);
        check(UserMarked.class.getAnnotation(annotation17shadow.Target.class).value() == 9);
        check(AliasedUse.class.getAnnotation(AliasedMark.class).mode() == Mode.SECOND);
        check(AliasedMark.class.getAnnotation(java.lang.annotation.Retention.class).value() == RetentionPolicy.RUNTIME);
        check(Arrays.equals(AliasedMark.class.getAnnotation(java.lang.annotation.Target.class).value(), new ElementType[] {ElementType.TYPE}));
        System.out.println("OK:" + count);
    }
}
