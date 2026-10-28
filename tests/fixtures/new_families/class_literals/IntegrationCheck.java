package newfamilies.integration;

import kotlin.jvm.JvmClassMappingKt;
import kotlin.reflect.KClass;

public final class IntegrationCheck {
    private static int checks;
    private static void equal(Object actual, Object expected) {
        if (!expected.equals(actual)) throw new AssertionError("expected " + expected + ", actual " + actual);
        checks++;
    }
    private static void token(KClass<?> actual, Class<?> expected) {
        equal(JvmClassMappingKt.getJavaClass(actual), expected);
    }
    public static void main(String[] args) {
        token(IntegrationKt.longArrayType(), long[].class);
        token(IntegrationKt.booleanArrayType(), boolean[].class);
        token(IntegrationKt.floatArrayType(), float[].class);
        token(IntegrationKt.doubleArrayType(), double[].class);
        equal(IntegrationKt.longArrayBound(new long[] {Long.MIN_VALUE}), long[].class);
        token(IntegrationKt.nestedDoubleArrayType(), double[][].class);
        equal(IntegrationKt.scalarAbi(new SourceScalar()), Integer.class);
        equal(IntegrationKt.sourceContractAbi(new SourceScalar()), Integer.class);
        try {
            IntegrationKt.scalarAbi(new SourceScalar() { public Integer item() { return null; } });
            throw new AssertionError("missing generic override null failure");
        } catch (NullPointerException expected) { checks++; }
        equal(IntegrationKt.sourceContractAbi(new SourceContract<Integer>() { public Integer item() { return 900; } }), Integer.class);
        equal(IntegrationKt.subjectless("A\ud83d\ude00"), 3);
        equal(IntegrationKt.subjectless(null), 0);
        equal(IntegrationKt.subjectless(10), 0);
        equal(IntegrationKt.subjectlessStatement("A\ud83d\ude00"), 3);
        equal(IntegrationKt.subjectlessStatement(null), -1);
        equal(IntegrationKt.subjectlessStatement(10), -1);
        equal(IntegrationKt.explicitFalse("A"), 1);
        equal(IntegrationKt.explicitFalse(null), 0);
        equal(IntegrationKt.explicitFalse(10), 0);
        System.out.println("checks=" + checks);
    }
}
