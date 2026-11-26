package arrays;
import android.annotation.NonNull;
public class ArraysApi {
    public static int count(@NonNull Object[] values) { return values.length; }
    public static String head(@NonNull String[] values) { return values[0]; }
}
