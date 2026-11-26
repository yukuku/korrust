package nullablevalues.consumer;
public final class Calls {
    static void equal(Object expected, Object actual) { if (!expected.equals(actual)) throw new AssertionError(expected + " != " + actual); }
    public static void main(String[] args) {
        equal(11, ConsumerKt.direct()); equal(11, ConsumerKt.local()); equal(7, ConsumerKt.property()); equal(23, ConsumerKt.constructed());
        equal(11, ConsumerKt.generic()); equal(11, ConsumerKt.container()); equal(13, ConsumerKt.defaultValue()); equal(19, ConsumerKt.optionalDefault()); equal(-1, ConsumerKt.explicitNull());
        equal(17L, ConsumerKt.wide()); equal("text", ConsumerKt.reference()); equal("absent", ConsumerKt.referenceNullSentinel()); equal("present", ConsumerKt.referencePresent());
        equal("wrapped:null", ConsumerKt.nullCarrier()); equal("wrapped:null", ConsumerKt.nullCarrierProperty()); equal("wrapped:null", ConsumerKt.nullCarrierDefault()); equal("wrapped:null", ConsumerKt.nullCarrierOptionalDefault()); equal("absent", ConsumerKt.nullCarrierExplicitNull()); equal("wrapped:null", ConsumerKt.nullCarrierGeneric()); equal("wrapped:null", ConsumerKt.nullCarrierContainer());
        equal("wrapped:null", ConsumerKt.numberCarrier()); equal("wrapped:null", ConsumerKt.numberCarrierProperty()); equal("wrapped:null", ConsumerKt.numberCarrierDefault()); equal("wrapped:null", ConsumerKt.numberCarrierOptionalDefault()); equal("absent", ConsumerKt.numberCarrierExplicitNull()); equal("wrapped:null", ConsumerKt.numberCarrierGeneric()); equal("wrapped:null", ConsumerKt.numberCarrierContainer());
        equal("wrapped:null", ConsumerKt.constructorTextDefault()); equal("wrapped:null", ConsumerKt.constructorNumberDefault());
        equal("wrapped:null", ConsumerKt.mixedDefault()); equal("absent", ConsumerKt.genericSentinelDefault()); equal("wrapped:null", ConsumerKt.genericSentinelProvided());
        equal("wrapped:null", ConsumerKt.constructorProvidedSentinel()); equal("wrapped:null", ConsumerKt.providedSentinelDefault());
        System.out.println("nullable-value-coercions-ok");
    }
}
