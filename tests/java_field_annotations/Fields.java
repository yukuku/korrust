package fieldannotations;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

public class Fields {
    @Audit({"static", "text"}) public static String text = "start";
    @Audit({"instance"}) public String item = "item";
    @Audit({"nonnull"}) @NotNull public static String nonnull = "nonnull";
    @Audit({"nullable"}) @Nullable public static String nullable = null;
    @Semantic public static String blocked = "blocked";
    @Audit({"primitive"}) public static int unsupportedPrimitive = 4;
}
