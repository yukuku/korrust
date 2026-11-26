package beans;
import org.jspecify.annotations.*;

public class Base {
    private String name = "start";
    private String optional;
    private boolean ready;
    public int reads;
    public int writes;
    public @NonNull String getName() { reads++; return name; }
    public void setName(@NonNull String name) { writes++; this.name = name; }
    public @Nullable String getOptional() { return optional; }
    public void setOptional(@Nullable String optional) { this.optional = optional; }
    public boolean isReady() { return ready; }
    public void setReady(boolean ready) { this.ready = ready; }
    public @NonNull String getURL() { return "url"; }
    public @NonNull String getReadOnly() { return "base"; }
}
