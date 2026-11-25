package handlers

import "github.com/labstack/echo/v4"

type Items struct{}

func (Items) List(c echo.Context) error { return nil }
func (Items) Show(c echo.Context) error { return nil }
func Health(c echo.Context) error       { return nil }
