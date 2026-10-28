package newfamilies.strings;

public final class StringsCheck {
    private static int checks;
    private static void equal(Object expected, Object actual) {
        if (!java.util.Objects.equals(expected, actual)) throw new AssertionError("Expected " + expected + ", actual " + actual);
        checks++;
    }
    public static void main(String[] args) {
        equal("\\n\\u0041\\t\\", RawStringsKt.plain());
        equal("😀é𝄞", RawStringsKt.unicode());
        equal(5, RawStringsKt.unicode().length());
        equal("|\"|\"\"|\"\"\"|\" a \"", RawStringsKt.quotes());
        equal("\n  first\n  second\n", RawStringsKt.multiline());
        equal("A:5:\\$name:$: $ 9", RawStringsKt.templates("A", 4));
        equal("abc", RawStringsKt.nested());
        equal("3", RawStringsKt.expressionLines());
        equal("\nA\t\\\"$😀", RawStringsKt.ordinary());
        equal("one \" two \"\" end", RawStringsKt.shortQuotes());
        equal("3", RawStringsKt.nestedComment());
        equal("ends\\", RawStringsKt.trailingSlash());
        equal("[null]", RawStringsKt.nullValue(null));
        equal("[hello]", RawStringsKt.nullValue("hello"));
        try { RawStringsKt.throwing(); throw new AssertionError("Expected exception"); }
        catch (IllegalArgumentException e) { equal("raw", e.getMessage()); }
        equal("1:2:3", RawStringsKt.ordered());
        equal(3, RawStringsKt.navigation("abcde"));
        equal(null, RawStringsKt.safeNavigation(null));
        equal(3, RawStringsKt.safeNavigation("abc"));
        equal(7, RawStringsKt.arithmetic());
        equal(1, RawStringsKt.statementBoundary());
        equal(1, RawStringsKt.lambdaBoundary());
        equal(30, RawStringsKt.indexing(new int[]{10, 20, 30}));
        equal(-2, RawStringsKt.unary());
        equal(true, RawStringsKt.booleans());
        equal("fallback", RawStringsKt.elvis(null));
        equal("x", RawStringsKt.elvis("x"));
        equal("x", RawStringsKt.casting("x"));
        equal(null, RawStringsKt.casting(5));
        equal(4, RawStringsKt.comments("abcd"));
        equal("a\nb\nc\nd", RawLineEndingsKt.rawLineEndings());
        equal(3, RawLineEndingsKt.crExpression());
        equal(3, RawLineEndingsKt.crComment());
        equal(3, RawStringsKt.commentArithmetic());
        equal("\\n\\u0041", RawStringsKt.RAW_LITERAL);
        equal("$", RawStringsKt.charTemplate('$'));
        equal(new String(new char[]{'\uD800'}), RawStringsKt.charTemplate('\uD800'));
        equal("null", RawStringsKt.nullableCharTemplate(null));
        equal("Z", RawStringsKt.nullableCharTemplate('Z'));
        equal(0, RawStringsKt.ignoredTrailingLambda());
        equal(7, RawStringsKt.executedTrailingLambda());
        equal(8, RawStringsKt.labeledTrailingLambda());
        equal("generic", RawStringsKt.genericTrailingLambda());
        System.out.println("checks=" + checks);
    }
}
