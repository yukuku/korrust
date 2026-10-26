import java.util.Objects;
import java.util.function.Supplier;
import compatibility.calls.PlatformJoinsKt;
import compatibility.calls.lib.CallEffects;

public final class PlatformJoinCheck {
    static void value(String expected, String trace, Supplier<String> call) {
        CallEffects.reset();
        String actual = call.get();
        if (!Objects.equals(actual, expected) || !CallEffects.trace().equals(trace)) {
            throw new AssertionError(actual + ":" + CallEffects.trace() + " != " + expected + ":" + trace);
        }
    }
    static void nullFailure(String trace, Supplier<String> call) {
        CallEffects.reset();
        try {
            call.get();
        } catch (NullPointerException expected) {
            if (!CallEffects.trace().equals(trace)) throw new AssertionError(CallEffects.trace() + " != " + trace);
            return;
        }
        throw new AssertionError("Expected a null failure: " + CallEffects.trace());
    }
    public static String check() {
        value(null, "CP", () -> PlatformJoinsKt.platformFirstNullable(true, true));
        value("value", "CP", () -> PlatformJoinsKt.platformFirstNullable(true, false));
        value("known", "CK", () -> PlatformJoinsKt.platformFirstNullable(false, true));
        value(null, "CP", () -> PlatformJoinsKt.platformLastNullable(false, true));
        value("value", "CP", () -> PlatformJoinsKt.platformLastNullable(false, false));
        value("known", "CK", () -> PlatformJoinsKt.platformLastNullable(true, true));
        value(null, "CP", () -> PlatformJoinsKt.platformNullableFirst(true, true, false));
        value("value", "CP", () -> PlatformJoinsKt.platformNullableFirst(true, false, false));
        value(null, "CN", () -> PlatformJoinsKt.platformNullableFirst(false, false, true));
        value("maybe", "CN", () -> PlatformJoinsKt.platformNullableFirst(false, true, false));
        value(null, "CP", () -> PlatformJoinsKt.nullablePlatformFirst(false, true, false));
        value("value", "CP", () -> PlatformJoinsKt.nullablePlatformFirst(false, false, false));
        value(null, "CN", () -> PlatformJoinsKt.nullablePlatformFirst(true, false, true));
        value("maybe", "CN", () -> PlatformJoinsKt.nullablePlatformFirst(true, true, false));
        nullFailure("CP", () -> PlatformJoinsKt.platformFirstRequired(true, true));
        value("value9", "CPLU", () -> PlatformJoinsKt.platformFirstRequired(true, false));
        value("known9", "CKLU", () -> PlatformJoinsKt.platformFirstRequired(false, true));
        nullFailure("CP", () -> PlatformJoinsKt.platformLastRequired(false, true));
        value("value9", "CPLU", () -> PlatformJoinsKt.platformLastRequired(false, false));
        value("known9", "CKLU", () -> PlatformJoinsKt.platformLastRequired(true, true));
        nullFailure("CP", () -> PlatformJoinsKt.platformNullableAsserted(true, true, false));
        nullFailure("CN", () -> PlatformJoinsKt.platformNullableAsserted(false, false, true));
        value("maybe9", "CNLU", () -> PlatformJoinsKt.platformNullableAsserted(false, true, false));
        nullFailure("CP", () -> PlatformJoinsKt.nullablePlatformAsserted(false, true, false));
        nullFailure("CN", () -> PlatformJoinsKt.nullablePlatformAsserted(true, false, true));
        value("maybe9", "CNLU", () -> PlatformJoinsKt.nullablePlatformAsserted(true, true, false));
        value("known", "CK", PlatformJoinsKt::platformJoins);
        return "known";
    }
}
