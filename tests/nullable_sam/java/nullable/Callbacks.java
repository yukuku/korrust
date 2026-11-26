package nullable;
import org.jetbrains.annotations.Nullable;
public final class Callbacks {
    public interface Listener { int apply(int value); }
    public interface Mapper<T> { T map(T value); }
    public interface Event { void run(@Nullable String value); }
    public interface Bad { int first(); int second(); }
    private final Listener listener;
    public Callbacks(@Nullable Listener listener) { this.listener = listener; }
    public int invoke(int value) { return listener == null ? -1 : listener.apply(value); }
    public static int call(@Nullable Listener listener, int value) { return listener == null ? -1 : listener.apply(value); }
    public static String map(@Nullable Mapper<String> mapper, String value) { return mapper == null ? "none" : mapper.map(value); }
    public static void event(@Nullable Event event, @Nullable String value) { if (event != null) event.run(value); }
    public static int bad(@Nullable Bad bad) { return 0; }
}
