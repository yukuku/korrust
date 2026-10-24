import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import yuku.alkitab.base.storage.InternalSingleChapterVerses;
import yuku.alkitab.base.storage.VerseTextDecoder;
import yuku.alkitab.base.widget.ReaderGestureActions;
import yuku.alkitab.model.SingleChapterVerses;
import yuku.alkitab.songs.SongTextZoomable;
import yuku.alkitab.yes1.Yes1SingleChapterVerses;
import simple.mutable.MutableInherited;
import simple.mutable.MutableOverride;
import simple.mutable.MutableSurface;
import simple.mutable.ReadOnlyBase;

public final class SimpleAndroidBibleCheck {
    private static final class Mutable implements MutableInherited, MutableOverride, MutableSurface.Nested, SongTextZoomable {
        private int count, item, value, zoom;
        private byte tiny;
        private char character;
        private boolean ready;
        private float amount;
        private String label, isTitle;
        private int[] ints;
        private byte[] bytes;
        private char[] chars;
        private String[] lines;
        public int getCount() { return count; }
        public void setCount(int next) { count = next; }
        public byte getTiny() { return tiny; }
        public void setTiny(byte next) { tiny = next; }
        public char getCharacter() { return character; }
        public void setCharacter(char next) { character = next; }
        public boolean isReady() { return ready; }
        public void setReady(boolean next) { ready = next; }
        public float getAmount() { return amount; }
        public void setAmount(float next) { amount = next; }
        public String getLabel() { return label; }
        public void setLabel(String next) { label = next; }
        public String isTitle() { return isTitle; }
        public void setTitle(String next) { isTitle = next; }
        public int[] getInts() { return ints; }
        public void setInts(int[] next) { ints = next; }
        public byte[] getBytes() { return bytes; }
        public void setBytes(byte[] next) { bytes = next; }
        public char[] getChars() { return chars; }
        public void setChars(char[] next) { chars = next; }
        public String[] getLines() { return lines; }
        public void setLines(String[] next) { lines = next; }
        public int getItem() { return item; }
        public void setItem(int next) { item = next; }
        public int getValue() { return value; }
        public void setValue(int next) { value = next; }
        public int getSongTextZoomPercent() { return zoom; }
        public void setSongTextZoomPercent(int next) { zoom = next; }
    }

    private static final class Actions implements ReaderGestureActions {
        final List<String> trace = new ArrayList<>();
        public void onFloaterAriSelected(int ari) { trace.add("ari:" + ari); }
        public void goToPreviousChapter() { trace.add("previous"); }
        public void goToNextChapter() { trace.add("next"); }
        public void applyPreferences() { trace.add("preferences"); }
        public void setFullScreenWithDrawerHandle(boolean yes) { trace.add("fullscreen:" + yes); }
    }

    private static final class Decoder implements VerseTextDecoder {
        public String[] separateIntoVerses(byte[] ba, boolean lowercased) {
            String[] result = new String[ba.length];
            for (int index = 0; index < ba.length; index++) result[index] = (lowercased ? "low:" : "raw:") + ba[index];
            return result;
        }
        public String makeIntoSingleString(byte[] ba, boolean lowercased) { return String.join("|", separateIntoVerses(ba, lowercased)); }
    }

    private static void mutable() throws Exception {
        Mutable implementation = new Mutable();
        SongTextZoomable zoom = implementation;
        MutableSurface surface = implementation;
        for (int next : new int[] {Integer.MIN_VALUE, -1, 0, 100, Integer.MAX_VALUE}) {
            zoom.setSongTextZoomPercent(next);
            InterfaceAssertions.record(zoom.getSongTextZoomPercent(), next);
            surface.setCount(next);
            InterfaceAssertions.record(surface.getCount(), next);
        }
        surface.setTiny(Byte.MIN_VALUE);
        surface.setCharacter('\udfff');
        surface.setReady(true);
        InterfaceAssertions.record(surface.getTiny(), Byte.MIN_VALUE);
        InterfaceAssertions.record(surface.getCharacter(), '\udfff');
        InterfaceAssertions.record(surface.isReady(), true);
        for (int bits : new int[] {0, Integer.MIN_VALUE, 0x7f800000, 0x7fc01234}) {
            surface.setAmount(Float.intBitsToFloat(bits));
            InterfaceAssertions.record(Float.floatToRawIntBits(surface.getAmount()), bits);
        }
        surface.setLabel("\u0000\ud800\udfff");
        InterfaceAssertions.record(surface.getLabel(), "\u0000\ud800\udfff");
        surface.setTitle("wire");
        InterfaceAssertions.record(surface.isTitle(), "wire");
        int[] ints = {Integer.MIN_VALUE, 0, Integer.MAX_VALUE};
        byte[] bytes = {Byte.MIN_VALUE, -1, 0, Byte.MAX_VALUE};
        char[] chars = {'\u0000', '\ud800', '\udfff', '\uffff'};
        String[] lines = {"", "\u0000", "\ud800", "\ud83d\ude00"};
        surface.setInts(ints);
        surface.setBytes(bytes);
        surface.setChars(chars);
        surface.setLines(lines);
        InterfaceAssertions.record(surface.getInts() == ints, true);
        InterfaceAssertions.record(surface.getBytes() == bytes, true);
        InterfaceAssertions.record(surface.getChars() == chars, true);
        InterfaceAssertions.record(surface.getLines() == lines, true);
        MutableInherited inherited = implementation;
        inherited.setCount(42);
        InterfaceAssertions.record(surface.getCount(), 42);
        InterfaceAssertions.record(inherited.readCount(), 42);
        InterfaceAssertions.record(inherited.readLabel(), "\u0000\ud800\udfff");
        MutableSurface.Nested nested = implementation;
        nested.setItem(-71);
        InterfaceAssertions.record(nested.getItem(), -71);
        MutableOverride writable = implementation;
        ReadOnlyBase readOnly = implementation;
        writable.setValue(17);
        InterfaceAssertions.record(readOnly.getValue(), 17);
        InterfaceAssertions.record(SongTextZoomable.class.getDeclaredMethods().length, 2);
        InterfaceAssertions.record(SongTextZoomable.class.getMethod("setSongTextZoomPercent", int.class).getReturnType(), void.class);
        InterfaceAssertions.record(SongTextZoomable.class.getMethod("setSongTextZoomPercent", int.class).getModifiers(), Modifier.PUBLIC | Modifier.ABSTRACT);
    }

    private static void chapters() throws Exception {
        for (boolean yes1 : new boolean[] {false, true}) {
            String[] verses = {"first", "\u0000\ud800", "\ud83d\ude00", null};
            SingleChapterVerses chapter = yes1 ? new Yes1SingleChapterVerses(verses) : new InternalSingleChapterVerses(verses);
            InterfaceAssertions.record(chapter.getVerseCount(), verses.length);
            InterfaceAssertions.record(chapter.getVerse(1), "\u0000\ud800");
            InterfaceAssertions.record(chapter.getVerse(3), null);
            verses[0] = "mutated";
            InterfaceAssertions.record(chapter.getVerse(0), "mutated");
            for (int index : new int[] {Integer.MIN_VALUE, -2, -1, 0, 1, 3, 4, 38, 39, Integer.MAX_VALUE - 1, Integer.MAX_VALUE}) {
                String[] expected = new String[1], actual = new String[1];
                String failure = InterfaceAssertions.failure(() -> expected[0] = verses[index]);
                InterfaceAssertions.record(InterfaceAssertions.failure(() -> actual[0] = chapter.getVerse(index)), failure);
                if (failure.equals("success")) InterfaceAssertions.record(actual[0], expected[0]);
                int one = index + 1;
                String[] labels = new String[40];
                for (int number = 0; number < labels.length; number++) labels[number] = Integer.toString(number);
                failure = InterfaceAssertions.failure(() -> expected[0] = one < 40 ? labels[one] : Integer.toString(one));
                InterfaceAssertions.record(InterfaceAssertions.failure(() -> actual[0] = chapter.getVerseNumberText(index)), failure);
                if (failure.equals("success")) InterfaceAssertions.record(actual[0], expected[0]);
            }
            SingleChapterVerses empty = yes1 ? new Yes1SingleChapterVerses(new String[0]) : new InternalSingleChapterVerses(new String[0]);
            InterfaceAssertions.record(empty.getVerseCount(), 0);
            InterfaceAssertions.record(InterfaceAssertions.failure(() -> empty.getVerse(0)).startsWith("java.lang.ArrayIndexOutOfBoundsException:"), true);
            InterfaceAssertions.record(InterfaceAssertions.failure(() -> {
                if (yes1) new Yes1SingleChapterVerses(null); else new InternalSingleChapterVerses(null);
            }).contains("java.lang.NullPointerException:"), true);
            java.lang.reflect.Field field = chapter.getClass().getDeclaredField("verses");
            InterfaceAssertions.record(field.getType(), String[].class);
            InterfaceAssertions.record(field.getModifiers(), Modifier.PRIVATE | Modifier.FINAL);
        }
    }

    public static void main(String[] args) throws Exception {
        mutable();
        chapters();
        Actions implementation = new Actions();
        ReaderGestureActions actions = implementation;
        actions.onFloaterAriSelected(Integer.MIN_VALUE);
        actions.goToPreviousChapter();
        actions.goToNextChapter();
        actions.applyPreferences();
        actions.setFullScreenWithDrawerHandle(true);
        actions.setFullScreenWithDrawerHandle(false);
        InterfaceAssertions.record(implementation.trace, Arrays.asList("ari:-2147483648", "previous", "next", "preferences", "fullscreen:true", "fullscreen:false"));
        VerseTextDecoder decoder = new Decoder();
        byte[] bytes = {Byte.MIN_VALUE, -1, 0, Byte.MAX_VALUE};
        InterfaceAssertions.record(Arrays.asList(decoder.separateIntoVerses(bytes, false)), Arrays.asList("raw:-128", "raw:-1", "raw:0", "raw:127"));
        InterfaceAssertions.record(decoder.makeIntoSingleString(bytes, true), "low:-128|low:-1|low:0|low:127");
        InterfaceAssertions.record(decoder.separateIntoVerses(new byte[0], false).length, 0);
        InterfaceAssertions.record(decoder.makeIntoSingleString(new byte[0], true), "");
        InterfaceAssertions.finish();
    }
}
