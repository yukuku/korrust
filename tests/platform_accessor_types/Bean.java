package fixtures;
import org.jspecify.annotations.*;
public class Bean { public int writes;
    public @NonNull String getWide() { return "str"; }
    public void setWide(@Nullable String v) { writes++; }
    public @Nullable String getNarrow() { return null; }
    public void setNarrow(@NonNull String v) { writes++; }
    public String getPlatform() { return ""; }
    public void setPlatform(@Nullable String v) { writes++; }
    public String getStrict() { return ""; } public void setStrict(@NonNull String v) { writes++; }
    public @NonNull String getResult() { return ""; } public void setResult(String v) { writes++; }
    public @Nullable String getMaybe() { return null; } public void setMaybe(String v) { writes++; }
    public String getParent() { return ""; }
    public void setParent(CharSequence v) { writes++; }
}
