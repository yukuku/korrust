import androidx.annotation.*;

public final class Migration {
    public static int calls;
    @RecentlyNullable public String nullableField;
    @Nullable public java.util.List<String> genericField;
    public static java.util.List<String> genericStaticField;
    @RecentlyNullable public static String staticField;
    @RecentlyNonNull public String nonNullField;
    @NonNull @RecentlyNullable public String strictNonNullField = "strict";
    @Nullable @RecentlyNonNull public String strictNullableField;
    @RecentlyNullable public static String nullable(boolean missing) { calls++; return missing ? null : "value"; }
    @RecentlyNonNull public static String nonNull(boolean missing) { calls++; return missing ? null : "value"; }
    @RecentlyNullable public static String accept(@RecentlyNonNull String value) { calls++; return value; }
    @RecentlyNonNull public static String nullableParameter(@RecentlyNullable String value) { calls++; return value; }
    @NonNull @RecentlyNullable public static String strictNonNull() { return "strict"; }
    @Nullable @RecentlyNonNull public static String strictNullable(boolean missing) { return missing ? null : "strict"; }
    public static String strictAccept(@NonNull @RecentlyNullable String value) { return value; }
    @RecentlyNullable public static <T> T identity(@RecentlyNonNull T value) { return value; }
}
