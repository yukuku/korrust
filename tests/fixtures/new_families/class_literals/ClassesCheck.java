package newfamilies.classes;

import kotlin.jvm.JvmClassMappingKt;
import kotlin.reflect.KClass;

public final class ClassesCheck {
    private static int checks;
    private static void equal(Object actual, Object expected) {
        if (!expected.equals(actual)) throw new AssertionError("expected " + expected + ", actual " + actual);
        checks++;
    }
    private static void token(KClass<?> actual, Class<?> expected) {
        equal(JvmClassMappingKt.getJavaClass(actual), expected);
    }
    public static void main(String[] args) throws Exception {
        equal(ClassesKt.class.getMethod("classMarked").getAnnotation(newfamilies.dependency.ClassMark.class).value(), String.class);
        token(ClassesKt.shortType(), short.class);
        equal(ClassesKt.class.getMethod("shortClassMarked").getAnnotation(newfamilies.dependency.ClassMark.class).value(), short.class);
        token(ClassesKt.shortBoxedArrayType(), Short[].class);
        token(ClassesKt.initializationType(), newfamilies.dependency.InitializationProbe.class);
        equal(newfamilies.dependency.InitializationTrace.initializations, 0);
        equal(ClassesKt.initializationJava(), newfamilies.dependency.InitializationProbe.class);
        equal(newfamilies.dependency.InitializationTrace.initializations, 0);
        token(ClassesKt.initializationBound(), newfamilies.dependency.InitializationProbe.class);
        equal(newfamilies.dependency.InitializationTrace.initializations, 1);
        equal(newfamilies.dependency.InitializationTrace.creations, 1);
        token(ClassesKt.publicLibraryType(), newfamilies.dependency.PublicKotlinType.class);
        token(ClassesKt.publicLibraryNested(), newfamilies.dependency.PublicOuter.PublicNested.class);
        token(ClassesKt.booleanType(), boolean.class);
        token(ClassesKt.byteType(), byte.class);
        token(ClassesKt.charType(), char.class);
        token(ClassesKt.intType(), int.class);
        token(ClassesKt.longType(), long.class);
        token(ClassesKt.floatType(), float.class);
        token(ClassesKt.doubleType(), double.class);
        token(ClassesKt.stringType(), String.class);
        token(ClassesKt.anyType(), Object.class);
        token(ClassesKt.sourceType(), FixtureType.class);
        token(ClassesKt.importedType(), StringBuilder.class);
        token(ClassesKt.genericType(), java.util.ArrayList.class);
        token(ClassesKt.listType(), java.util.List.class);
        token(ClassesKt.intArrayType(), int[].class);
        token(ClassesKt.byteArrayType(), byte[].class);
        token(ClassesKt.charArrayType(), char[].class);
        token(ClassesKt.stringArrayType(), String[].class);
        token(ClassesKt.boxedArrayType(), Integer[].class);
        token(ClassesKt.nestedArrayType(), int[][].class);
        token(ClassesKt.nullableElementArrayType(), String[].class);
        token(ClassesKt.valueShadow(), Integer.class);
        token(ClassesKt.localShadow(), String.class);
        token(ClassesKt.boundAny(Integer.valueOf(5)), Integer.class);
        token(ClassesKt.boundAny(new FixtureType(3)), FixtureType.class);
        token(ClassesKt.boundInterface(new StringBuilder()), StringBuilder.class);
        token(ClassesKt.boundInt(4), int.class);
        token(ClassesKt.boundString("value"), String.class);
        token(ClassesKt.boundArray(new String[] {"value"}), String[].class);
        token(ClassesKt.asserted("nonnull"), String.class);
        try { ClassesKt.asserted(null); throw new AssertionError("missing null failure"); }
        catch (NullPointerException expected) { checks++; }
        ClassesKt.setCalls(0);
        token(ClassesKt.intEffect(), int.class);
        equal(ClassesKt.getCalls(), 1);
        token(ClassesKt.textEffect(), String.class);
        equal(ClassesKt.getCalls(), 2);
        try { ClassesKt.failEffect(); throw new AssertionError("missing receiver failure"); }
        catch (IllegalStateException expected) { equal(expected.getMessage(), "class-effect"); }
        equal(ClassesKt.getCalls(), 3);
        equal(ClassesKt.javaInt(), int.class);
        equal(ClassesKt.javaString(), String.class);
        equal(ClassesKt.javaArray(), String[].class);
        equal(ClassesKt.javaBound(Integer.valueOf(9)), Integer.class);
        equal(ClassesKt.javaBoundInt(9), int.class);
        equal(ClassesKt.javaBoxedInt(), Integer.class);
        equal(ClassesKt.javaPrimitiveInt(), int.class);
        if (ClassesKt.javaPrimitiveString() != null) throw new AssertionError("reference primitive type");
        checks++;
        token(ClassesKt.kotlinClass(String.class), String.class);
        equal(ClassesKt.relayType(), int.class);
        equal(ClassesKt.relayBound(Integer.valueOf(2)), Integer.class);
        equal(ClassesKt.simpleName(), "FixtureType");
        equal(ClassesKt.className(Integer.valueOf(2)), "java.lang.Integer");
        equal(ClassesKt.threadName(new Thread("family-six")), "family-six");
        equal(ClassesKt.intType(), ClassesKt.boundAny(Integer.valueOf(7)));
        token(ClassesKt.nestedType(), Outer.Nested.class);
        equal(ClassesKt.erasedSource(), Integer.class);
        equal(ClassesKt.erasedLibrary(), Integer.class);
        equal(ClassesKt.erasedProperty(java.util.List.of(42)), Integer.class);
        equal(ClassesKt.erasedInvocation(() -> 42), Integer.class);
        equal(ClassesKt.assertedInt(42), Integer.class);
        try { ClassesKt.assertedInt(null); throw new AssertionError("missing boxed null failure"); }
        catch (NullPointerException expected) { checks++; }
        equal(ClassesKt.castInt(42), Integer.class);
        try { ClassesKt.castInt("wrong"); throw new AssertionError("missing primitive cast failure"); }
        catch (ClassCastException expected) { checks++; }
        equal(ClassesKt.listElement(java.util.List.of(42)), Integer.class);
        equal(ClassesKt.boxedArrayElement(new Integer[] {42}), Integer.class);
        equal(ClassesKt.primitiveArrayElement(new int[] {42}), int.class);
        equal(ClassesKt.conditional(true), int.class);
        equal(ClassesKt.conditional(false), int.class);
        equal(ClassesKt.erasedUnchecked(), String.class);
        equal(ClassesKt.storedPrimitive(), int.class);
        token(ClassesKt.parenthesizedArray(), String[].class);
        token(ClassesKt.genericBound("bounded"), String.class);
        token(ClassesKt.genericBoundChain("chain"), String.class);
        try { ClassesKt.genericBound(null); throw new AssertionError("missing generic null failure"); }
        catch (NullPointerException expected) { checks++; }
        token(ClassesKt.qualifiedArray(), String[].class);
        token(ClassesKt.escapedBound("escaped"), String.class);
        System.out.println("checks=" + checks);
    }
}
