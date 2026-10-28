package newfamilies.constants;

public final class ConstantsCheck {
    private static int checks;
    private static void field(String name, Object expected) throws Exception {
        Object actual = ConstantsKt.class.getField(name).get(null);
        if (!java.util.Objects.equals(expected, actual)) throw new AssertionError(name + ": " + expected + " != " + actual);
        checks++;
    }
    private static void equal(Object expected, Object actual) {
        if (!java.util.Objects.equals(expected, actual)) throw new AssertionError(expected + " != " + actual);
        checks++;
    }
    public static void main(String[] args) throws Exception {
        equal(false, newfamilies.library.InitializationTrace.initialized);
        field("BASE", 42);
        field("PARENTHESIZED", 1);
        field("FORWARD", 45);
        field("LATER", 3);
        field("INT_WRAP", Integer.MIN_VALUE);
        field("LONG_WRAP", Long.MIN_VALUE);
        field("INT_DIV_MIN", Integer.MIN_VALUE);
        field("INT_REM_MIN", 0);
        field("LONG_DIV_MIN", Long.MIN_VALUE);
        field("LONG_REM_MIN", 0L);
        field("DECLARED_LONG", 3L);
        field("INT_OVERFLOW_TO_LONG", -2147483648L);
        field("INT_DIV_TO_LONG", -2147483648L);
        field("MIXED_LONG", 22L);
        field("FLOAT_RESULT", 3.25f);
        field("DOUBLE_RESULT", 3.5);
        field("FLOAT_NAN", Float.NaN);
        field("DOUBLE_NAN", Double.NaN);
        field("FLOAT_INFINITY", Float.POSITIVE_INFINITY);
        field("DOUBLE_INFINITY", Double.NEGATIVE_INFINITY);
        field("FLOAT_NEGATIVE_ZERO", -0.0f);
        field("DOUBLE_NEGATIVE_ZERO", -0.0);
        field("NAN_EQUALITY", false);
        field("ZERO_EQUALITY", true);
        field("UTF16_ORDER", true);
        field("CHAR_WRAP", '\u0000');
        field("CHAR_DISTANCE", 25);
        field("CHARACTER", '`');
        field("TEXT", "ab25");
        field("TEMPLATE", "x42:" + new String(new char[]{'\uD800'}) + ":null");
        field("TRUE_VALUE", true);
        field("FALSE_LAZY", false);
        field("TRUE_LAZY", true);
        field("STRING_ORDER", true);
        field("BOOLEAN_ORDER", true);
        field("FLOAT_COMPARE", false);
        field("IMPORTED_VALUE", 21);
        field("IMPORTED_WIDE", 9007199254740994L);
        field("IMPORTED_TEXT", "library:9");
        field("IMPORTED_BOOLEAN", true);
        field("IMPORTED_CHAR", new String(new char[]{'\uD800'}));
        field("IMPORTED_FLOAT", 3.0f);
        field("IMPORTED_DOUBLE", -0.0);
        field("SOURCE_IMPORT", 14);
        field("OBJECT_VALUE", 95);
        equal(47, Holder.class.getField("TOTAL").get(null));
        equal(48, Holder.Nested.class.getField("NEXT").get(null));
        equal(45, ConstantsKt.annotated());
        equal("ab25", ConstantsKt.returnedText());
        newfamilies.library.RuntimeConstant annotation = ConstantsKt.class.getMethod("annotated").getAnnotation(newfamilies.library.RuntimeConstant.class);
        equal(84, annotation.answer());
        equal("answer:42", annotation.text());
        equal(true, annotation.flag());
        equal(6, Holder.class.getMethod("marked").getAnnotation(newfamilies.library.RuntimeConstant.class).answer());
        equal(10, ConstantsKt.class.getMethod("annotationForward").getAnnotation(newfamilies.library.RuntimeConstant.class).answer());
        equal(8, newfamilies.priority.ImportChoiceKt.class.getField("CHOICE").get(null));
        equal(8, newfamilies.samefile.SameFileChoiceKt.class.getField("CHOICE").get(null));
        equal(3, newfamilies.samefile.Member.class.getField("CHOICE").get(null));
        equal("chara", ConstantsKt.charConcat('a'));
        equal(false, newfamilies.library.InitializationTrace.initialized);
        System.out.println("checks=" + checks);
    }
}
