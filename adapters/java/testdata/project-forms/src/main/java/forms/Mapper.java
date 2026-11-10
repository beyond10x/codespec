package forms;

import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.ext.ExceptionMapper;
import jakarta.ws.rs.ext.Provider;

public class Mapper {
    long n;

    @Provider
    public static class Errors implements ExceptionMapper<Exception> { // refused: ExceptionMapper
        @Override
        public Response toResponse(Exception e) {
            return Response.serverError().build();
        }
    }
}
