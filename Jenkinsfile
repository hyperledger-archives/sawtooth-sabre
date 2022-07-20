#!groovy

// Copyright 2017 Intel Corporation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// ------------------------------------------------------------------------------

pipeline {
    agent {
        node {
            label 'master'
            customWorkspace "workspace/${env.BUILD_TAG}"
        }
    }

    triggers {
        cron(env.BRANCH_NAME == '0-1' ? 'H 3 * * *' : '')
    }

    options {
        timestamps()
        buildDiscarder(logRotator(daysToKeepStr: '31'))
    }

    environment {
        ISOLATION_ID = sh(returnStdout: true, script: 'printf $BUILD_TAG | sha256sum | cut -c1-64').trim()
        COMPOSE_PROJECT_NAME = sh(returnStdout: true, script: 'printf $BUILD_TAG | sha256sum | cut -c1-64').trim()
    }

    stages {
        stage('Check Whitelist') {
            steps {
                readTrusted 'bin/whitelist'
                sh './bin/whitelist "$CHANGE_AUTHOR" /etc/jenkins-authorized-builders'
            }
            when {
                not {
                    branch '0-1'
                }
            }
        }

        stage('Check for Signed-Off Commits') {
            steps {
                sh '''#!/bin/bash -l
                    if [ -v CHANGE_URL ] ;
                    then
                        temp_url="$(echo $CHANGE_URL |sed s#github.com/#api.github.com/repos/#)/commits"
                        pull_url="$(echo $temp_url |sed s#pull#pulls#)"

                        IFS=$'\n'
                        for m in $(curl -s "$pull_url" | grep "message") ; do
                            if echo "$m" | grep -qi signed-off-by:
                            then
                              continue
                            else
                              echo "FAIL: Missing Signed-Off Field"
                              echo "$m"
                              exit 1
                            fi
                        done
                        unset IFS;
                    fi
                '''
            }
        }

        stage('Fetch Tags') {
            steps {
                sh 'git fetch --tag'
            }
        }

        stage('Build Sabre') {
            steps {
                sh 'docker-compose -f docker-compose-installed.yaml build sabre-cli'
                sh 'docker-compose -f docker-compose-installed.yaml build sabre-tp'
            }
        }

        stage('Test Sabre') {
            steps {
                sh 'docker-compose -f docker-compose.yaml -f integration/sabre_test.yaml up --build --abort-on-container-exit --exit-code-from test_sabre'
            }
        }

        stage('Create Git Archive') {
            steps {
                sh '''
                    REPO=$(git remote show -n origin | grep Fetch | awk -F'[/.]' '{print $6}')
                    VERSION=`git describe --dirty`
                    git archive HEAD --format=zip -9 --output=$REPO-$VERSION.zip
                    git archive HEAD --format=tgz -9 --output=$REPO-$VERSION.tgz
                '''
            }
        }

        stage('Build Archive Artifacts') {
            steps {
                sh 'mkdir -p build/debs'
                sh 'docker run --rm -v $(pwd)/build/debs:/build/debs --entrypoint "/bin/bash" sawtooth-sabre-cli:${ISOLATION_ID} "-c" "cp /tmp/*.deb /build/debs"'
                sh 'docker run --rm -v $(pwd)/build/debs:/build/debs --entrypoint "/bin/bash" sawtooth-sabre-tp:${ISOLATION_ID} "-c" "cp /tmp/*.deb /build/debs"'
            }
        }
    }
    post {
        always {
            sh 'docker-compose -f docker-compose.yaml -f integration/sabre_test.yaml down'
        }
        success {
            archiveArtifacts artifacts: '*.tgz, *.zip, build/debs/*.deb'
        }
        aborted {
            error "Aborted, exiting now"
        }
        failure {
            error "Failed, exiting now"
        }
    }
}
