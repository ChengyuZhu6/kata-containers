// Copyright (c) 2018 HyperHQ Inc.
//
// SPDX-License-Identifier: Apache-2.0
//

package containerdshim

import (
	"context"
	"fmt"
	"io"
	"os"

	"github.com/containerd/containerd/namespaces"
	cdshim "github.com/containerd/containerd/runtime/v2/shim"
	"github.com/kata-containers/kata-containers/src/runtime/pkg/katautils"
	vci "github.com/kata-containers/kata-containers/src/runtime/virtcontainers"
	"github.com/sirupsen/logrus"
)

const (
	bufferSize = 32
	chSize     = 4
)

// IsWarmStartup checks if current shim was started in warm mode
// This leverages containerd's built-in warmstart action support
func IsWarmStartup() bool {
	// Check if we were started with "warmstart" action
	if len(os.Args) > 1 {
		return os.Args[1] == "warmstart"
	}
	return false
}

// GetWarmID gets warm shim ID from command line arguments
func GetWarmID() string {
	// In warm mode, the ID is typically passed as argument
	if IsWarmStartup() && len(os.Args) > 2 {
		// Look for -id flag value
		for i, arg := range os.Args {
			if arg == "-id" && i+1 < len(os.Args) {
				return os.Args[i+1]
			}
		}
	}
	return ""
}

// LogWarmStartup logs when shim starts in warm mode
func LogWarmStartup(warmID, namespace string) {
	shimLog.WithFields(logrus.Fields{
		"warm_id":   warmID,
		"namespace": namespace,
		"mode":      "warm",
	}).Info("kata shim started in warm mode")
}

// isWarmBound returns whether this shim was bound from warm state
func (s *service) isWarmBound() bool {
	// For kata, we consider a shim warm-bound if it was started with warmstart action
	// This is simpler than runc's approach and leverages containerd's existing infrastructure
	return IsWarmStartup()
}

// NewWarm creates a new warm shim service
// This is called when the shim is started in warm mode
func NewWarm(ctx context.Context, id string, publisher cdshim.Publisher, shutdown func()) (cdshim.Shim, error) {
	// Log that we're starting in warm mode
	if warmID := GetWarmID(); warmID != "" {
		LogWarmStartup(warmID, id)
	} else {
		shimLog.WithField("id", id).Info("kata shim started in warm mode")
	}

	// In warm mode, we create a service that's ready to be bound later
	// but doesn't immediately try to initialize kata sandbox/containers
	return newWarmService(ctx, id, publisher, shutdown)
}

// newWarmService creates a service instance for warm mode
// This avoids early initialization that might fail without proper container context
func newWarmService(ctx context.Context, id string, publisher cdshim.Publisher, shutdown func()) (cdshim.Shim, error) {
	// Set up logging similar to New() but without kata-specific initialization
	logrus.SetOutput(io.Discard)
	opts := ctx.Value(cdshim.OptsKey{}).(cdshim.Opts)
	if !opts.Debug {
		logrus.SetLevel(logrus.WarnLevel)
	}
	vci.SetLogger(ctx, shimLog)
	katautils.SetLogger(ctx, shimLog, shimLog.Logger.Level)

	ns, found := namespaces.Namespace(ctx)
	if !found {
		return nil, fmt.Errorf("shim namespace cannot be empty")
	}

	s := &service{
		id:         id,
		pid:        uint32(os.Getpid()),
		ctx:        ctx,
		containers: make(map[string]*container),
		events:     make(chan interface{}, chSize),
		ec:         make(chan exit, bufferSize),
		cancel:     shutdown,
		namespace:  ns,
		warmBound:  false, // Mark as not bound yet
	}

	go s.processExits()

	forwarder := s.newEventsForwarder(ctx, publisher)
	go forwarder.forward()

	return s, nil
}
