package genericfields;
import java.util.List;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
public class Cell<T> {
    public String label = "label";
    public T value;
    @Deprecated public T historical;
    private T hidden;
    public final T fixed;
    @Nullable public T optional;
    @NotNull public T required;
    public List<T> list;
    public T[] array;
    public volatile T published;
    public static List<String> global;
    public Cell(T value, T optional, List<T> list, T[] array) {
        this.value = value;
        this.fixed = value;
        this.optional = optional;
        this.required = value;
        this.list = list;
        this.array = array;
        this.published = value;
    }
}
