import java.util.*;
import android.annotation.*;
import org.jetbrains.annotations.*;
public class Lists {
    private static List<CharSequence> fresh() { return new ArrayList<>(); }
    @NonNull public static List<CharSequence> nonNull() { return fresh(); }
    @Nullable public static List<CharSequence> nullable(boolean missing) { return missing ? null : fresh(); }
    public static List<CharSequence> platform(boolean missing) { return missing ? null : fresh(); }
    @NonNull @ReadOnly public static List<CharSequence> readOnly() { return fresh(); }
    @NonNull @Mutable public static List<CharSequence> mutable() { return fresh(); }
    @ReadOnly public static List<CharSequence> platformReadOnly() { return fresh(); }
    @Mutable public static List<CharSequence> platformMutable() { return fresh(); }
    public static List<String> strings() { return new ArrayList<>(); }
    public static int accept(@NonNull List<CharSequence> value) { return value.size(); }
    public static int readonly(@ReadOnly @NonNull List<CharSequence> value) { return value.size(); }
    public static int mutableArg(@Mutable @NonNull List<CharSequence> value) { return value.size(); }
    public static List<? extends CharSequence> projected() { return fresh(); }
}
