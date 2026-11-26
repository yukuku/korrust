package constructors.consumer;
import constructors.library.JavaTypes;
public class Calls {
    public static void main(String[] args) {
        if (ConstructorsKt.javaPrimitive() != 7 || ConstructorsKt.kotlinPrimitive() != 8 || ConstructorsKt.numericBatch() != 4) throw new AssertionError();
        if (ConstructorsKt.<String>genericLambda().getTransform().invoke("ignored") != 6) throw new AssertionError();
        if (ConstructorsKt.valueClass() != 12) throw new AssertionError();
        if (!ConstructorsKt.javaOrder().equals("ABJ") || !ConstructorsKt.kotlinOrder().equals("AB:BAK")) throw new AssertionError();
        if (ConstructorsKt.nullableParameter().getValue() != null || ConstructorsKt.nullableParameter().getTag() != 7) throw new AssertionError();
        if (ConstructorsKt.javaNull().getValue() != null) throw new AssertionError();
        if (!ConstructorsKt.javaBox().equals("java")) throw new AssertionError();
        if (!ConstructorsKt.kotlinBox().equals("kotlin")) throw new AssertionError();
        if (!ConstructorsKt.inferredBox().getValue().equals("metadata")) throw new AssertionError();
        if (!ConstructorsKt.rebox("rebox").getValue().equals("rebox")) throw new AssertionError();
        if (!ConstructorsKt.collection().equals("collection")) throw new AssertionError();
        if (!ConstructorsKt.linked().equals("linked")) throw new AssertionError();
        if (!ConstructorsKt.bounded().equals("bound")) throw new AssertionError();
        if (ConstructorsKt.defaults().getValue() != null || ConstructorsKt.defaults().getTag() != 7) throw new AssertionError();
        if (ConstructorsKt.namedDefault().getTag() != 9) throw new AssertionError();
        if (!ConstructorsKt.named().getValue().equals("named")) throw new AssertionError();
        if (ConstructorsKt.nullable().getValue() != null) throw new AssertionError();
        if (!ConstructorsKt.context().getValue().isEmpty()) throw new AssertionError();
        if (ConstructorsKt.contextualLambda().getTransform().invoke("abc") != 3) throw new AssertionError();
        if (!ConstructorsKt.covariance().getValue().toString().equals("cov")) throw new AssertionError();
        if (ConstructorsKt.contravariance().tag() != 17) throw new AssertionError();
        if (ConstructorsKt.projected().getClass() != JavaTypes.Flipped.class) throw new AssertionError();
        if (!ConstructorsKt.shadowedParameter().text.equals("shadow")) throw new AssertionError();
        if (!ConstructorsKt.dependent().equals("dependent")) throw new AssertionError();
        if (!ConstructorsKt.dependentExpected().getValue().toString().equals("expected")) throw new AssertionError();
        if (ConstructorsKt.choice() != 2) throw new AssertionError();
        if (!ConstructorsKt.batch().equals("one")) throw new AssertionError();
        if (!ConstructorsKt.factory().equals("factory")) throw new AssertionError();
        if (!ConstructorsKt.shadow(value -> "shadow:" + value).equals("shadow:local")) throw new AssertionError();
        System.out.println("constructor-inference-ok");
    }
}
