package delegated;
public interface JavaDefaults { int value(); default int computed(){return value();} }
