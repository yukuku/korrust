package fixtures.io;

public final class IoTrace {
    public final byte[] bytes;
    public final char[] chars;
    public final int position;
    public final byte element;
    public final boolean failIndex;
    public final boolean failValue;
    public String trace = "";
    public IoTrace(byte[] bytes, char[] chars, int position, byte element, boolean failIndex, boolean failValue) {
        this.bytes = bytes;
        this.chars = chars;
        this.position = position;
        this.element = element;
        this.failIndex = failIndex;
        this.failValue = failValue;
    }
    public byte[] array() { trace += "A"; return bytes; }
    public char[] charArray() { trace += "C"; return chars; }
    public int index() { trace += "I"; if (failIndex) throw new IllegalStateException("index"); return position; }
    public byte value() { trace += "V"; if (failValue) throw new IllegalArgumentException("value"); return element; }
    public char charValue() { trace += "H"; if (failValue) throw new IllegalArgumentException("char"); return (char)(element & 255); }
    public int length() { trace += "L"; if (failValue) throw new IllegalArgumentException("length"); return 1; }
    public int subject() { trace += "S"; return position; }
    public int low() { trace += "L"; return 0; }
    public int high() { trace += "H"; return position; }
    public int forEnd() { trace += "E"; return position; }
    public int ticks() { return trace.length(); }
}
