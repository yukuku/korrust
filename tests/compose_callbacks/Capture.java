package composeprobe;
import androidx.compose.runtime.Composer;
import java.util.*;
public final class Capture {
    public static final List<Entry> entries=new ArrayList<>();
    public static final class Entry {
        public final String name;
        public final Composer composer;
        public final int[] words;
        Entry(String name, Composer composer, int[] words) { this.name=name; this.composer=composer; this.words=words; }
    }
    public static void record(String name, Composer composer, int[] words) { entries.add(new Entry(name,composer,words)); }
}
