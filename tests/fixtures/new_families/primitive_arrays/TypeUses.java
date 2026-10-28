package newfamilies.arrays;
import org.jspecify.annotations.NonNull;
import org.jspecify.annotations.Nullable;

public final class TypeUses {
    public static double @NonNull [] nonnull(double @NonNull [] value) { return value; }
    public static double @Nullable [] nullable(double @Nullable [] value) { return value; }
    public static @NonNull String text(@NonNull String value) { return value; }
    public static @Nullable String nullableText(@Nullable String value) { return value; }
    @org.jetbrains.annotations.NotNull
    public static double @NonNull [] combined(@org.jetbrains.annotations.NotNull double @NonNull [] value) { return value; }
    @org.jetbrains.annotations.Nullable
    public static double @Nullable [] combinedNullable(@org.jetbrains.annotations.Nullable double @Nullable [] value) { return value; }
    public static double @Unknown [] unknown(double @Unknown [] value) { return value; }
    public static @NonNull String[] nested(@NonNull String[] value) { return value; }
}
