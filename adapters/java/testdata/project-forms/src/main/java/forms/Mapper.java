package forms;

import jakarta.ws.rs.WebApplicationException;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.ext.ExceptionMapper;
import jakarta.ws.rs.ext.Provider;

public class Mapper {
    long n;

    @Provider
    public static class Errors implements ExceptionMapper<Exception> {
        @Override
        public Response toResponse(Exception e) {
            int code = 500;
            if (e instanceof WebApplicationException) { // refused: instanceof
                code = 404;
            }
            return Response.status(code).build();
        }

        static int status(Exception e) {
            return ((WebApplicationException) e).getResponse().getStatus(); // refused: cast
        }
    }

    public static class Unregistered implements ExceptionMapper<RuntimeException> { // refused: ExceptionMapper
        @Override
        public Response toResponse(RuntimeException e) {
            return Response.serverError().build();
        }
    }
}
