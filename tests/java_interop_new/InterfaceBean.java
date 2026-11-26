package newinterop;

public interface InterfaceBean {
    String getTitle();
    interface Writable extends InterfaceBean {
        void setTitle(String value);
    }
}
